use alloc::vec;
use alloc::vec::Vec;

use crate::bool_coder::BoolEncoder;
use crate::color::{convert, YuvPlanes};
use crate::prediction::{predict_chroma_dc, predict_luma_dc};
use crate::quantize::{dequantize_block, factors, quantize_block, QuantizationFactors};
use crate::residual::{MacroblockResidual, ResidualWriter, COEFF_UPDATE_PROBS};
use crate::transform::{clamped_add, forward_dct, forward_wht, inverse_dct, inverse_wht};
use crate::{Alpha, Filter, Options};

const DC_MODE: u8 = 0;

// RFC 6386 section 11.2 defines the key-frame luma mode tree and probabilities.
pub(crate) const KF_Y_MODE_TREE: [i8; 8] = [-4, 2, 4, 6, 0, -1, -2, -3];
pub(crate) const KF_Y_MODE_PROBS: [u8; 4] = [145, 156, 163, 128];

// RFC 6386 section 11.4 defines the chroma mode tree and probabilities.
pub(crate) const UV_MODE_TREE: [i8; 6] = [0, 2, -1, 4, -2, -3];
pub(crate) const KF_UV_MODE_PROBS: [u8; 3] = [142, 114, 183];

pub(crate) struct EncodedFrame {
    pub(crate) webp: Vec<u8>,
    #[cfg(test)]
    pub(crate) reconstruction: YuvPlanes,
}

pub(crate) fn encode(
    pixels: &[u8],
    width: usize,
    height: usize,
    bytes_per_pixel: usize,
    quantizer_index: u8,
    options: &Options,
) -> EncodedFrame {
    let source = convert(pixels, width, height, bytes_per_pixel);
    let macroblock_columns = width.div_ceil(16);
    let macroblock_rows = height.div_ceil(16);
    let macroblock_count = macroblock_columns * macroblock_rows;
    let mut reconstruction = YuvPlanes {
        y: vec![0; source.y.len()],
        u: vec![0; source.u.len()],
        v: vec![0; source.v.len()],
        y_stride: source.y_stride,
        chroma_stride: source.chroma_stride,
    };
    let mut first_partition = BoolEncoder::with_capacity(160 + macroblock_count.div_ceil(2));
    let mut second_partition = BoolEncoder::with_capacity(width * height);
    let mut residual_writer = ResidualWriter::new(macroblock_columns);
    let quantization = factors(quantizer_index);

    write_frame_header(&mut first_partition, quantizer_index, options.filter);
    for macroblock_y in 0..macroblock_rows {
        for macroblock_x in 0..macroblock_columns {
            first_partition.write_tree(&KF_Y_MODE_TREE, &KF_Y_MODE_PROBS, DC_MODE, 0);
            first_partition.write_tree(&UV_MODE_TREE, &KF_UV_MODE_PROBS, DC_MODE, 0);

            let luma_prediction = predict_luma_dc(
                &reconstruction.y,
                reconstruction.y_stride,
                macroblock_x,
                macroblock_y,
            );
            let u_prediction = predict_chroma_dc(
                &reconstruction.u,
                reconstruction.chroma_stride,
                macroblock_x,
                macroblock_y,
            );
            let v_prediction = predict_chroma_dc(
                &reconstruction.v,
                reconstruction.chroma_stride,
                macroblock_x,
                macroblock_y,
            );
            let mut residual = MacroblockResidual::default();
            analyze_luma(
                &source.y,
                &luma_prediction,
                &mut reconstruction.y,
                source.y_stride,
                (macroblock_x, macroblock_y),
                quantization,
                &mut residual,
            );
            analyze_chroma(
                &source.u,
                &u_prediction,
                &mut reconstruction.u,
                source.chroma_stride,
                (macroblock_x, macroblock_y),
                quantization,
                &mut residual.u,
            );
            analyze_chroma(
                &source.v,
                &v_prediction,
                &mut reconstruction.v,
                source.chroma_stride,
                (macroblock_x, macroblock_y),
                quantization,
                &mut residual.v,
            );
            residual_writer.write_macroblock(&mut second_partition, macroblock_x, &residual);
        }
    }

    let first_partition = first_partition.finish();
    let second_partition = second_partition.finish();
    let vp8 = assemble_vp8(width, height, &first_partition, &second_partition);
    EncodedFrame {
        webp: assemble_riff(&vp8, pixels, width, height, bytes_per_pixel, options),
        #[cfg(test)]
        reconstruction,
    }
}

fn write_frame_header(encoder: &mut BoolEncoder, quantizer_index: u8, filter: Filter) {
    let (level, sharpness) = match filter {
        Filter::Level { level, sharpness } => (level.min(63), sharpness.min(7)),
        Filter::Auto | Filter::Off => (0, 0),
    };

    // RFC 6386 sections 9 and 19.2 fix this field order.
    encoder.write_literal(0, 1);
    encoder.write_literal(0, 1);
    encoder.write_literal(0, 1);
    encoder.write_literal(0, 1);
    encoder.write_literal(u32::from(level), 6);
    encoder.write_literal(u32::from(sharpness), 3);
    encoder.write_literal(0, 1);
    encoder.write_literal(0, 2);
    encoder.write_literal(u32::from(quantizer_index), 7);
    for _ in 0..5 {
        encoder.write_literal(0, 1);
    }
    encoder.write_literal(1, 1);
    for probability in COEFF_UPDATE_PROBS {
        encoder.write_bool(probability, false);
    }
    encoder.write_literal(0, 1);
}

fn analyze_luma(
    source: &[u8],
    prediction: &[u8; 256],
    reconstruction: &mut [u8],
    stride: usize,
    macroblock: (usize, usize),
    quantization: QuantizationFactors,
    residual: &mut MacroblockResidual,
) {
    let (macroblock_x, macroblock_y) = macroblock;
    let mut dc_coefficients = [0; 16];
    for (block, (dc_coefficient, levels)) in dc_coefficients
        .iter_mut()
        .zip(residual.y.iter_mut())
        .enumerate()
    {
        let block_x = block % 4;
        let block_y = block / 4;
        let samples = residual_block(
            source,
            stride,
            (
                macroblock_x * 16 + block_x * 4,
                macroblock_y * 16 + block_y * 4,
            ),
            prediction,
            16,
            (block_x * 4, block_y * 4),
        );
        let mut coefficients = forward_dct(&samples);
        *dc_coefficient = coefficients[0];
        coefficients[0] = 0;
        *levels = quantize_block(&coefficients, quantization.y_dc, quantization.y_ac);
    }
    residual.y2 = quantize_block(
        &forward_wht(&dc_coefficients),
        quantization.y2_dc,
        quantization.y2_ac,
    );
    let reconstructed_dc = inverse_wht(&dequantize_block(
        &residual.y2,
        quantization.y2_dc,
        quantization.y2_ac,
    ));

    for (block, (dc_coefficient, levels)) in
        reconstructed_dc.iter().zip(residual.y.iter()).enumerate()
    {
        let block_x = block % 4;
        let block_y = block / 4;
        let mut coefficients = dequantize_block(levels, quantization.y_dc, quantization.y_ac);
        coefficients[0] = *dc_coefficient;
        write_reconstruction_block(
            reconstruction,
            stride,
            (
                macroblock_x * 16 + block_x * 4,
                macroblock_y * 16 + block_y * 4,
            ),
            prediction,
            16,
            (block_x * 4, block_y * 4),
            &inverse_dct(&coefficients),
        );
    }
}

fn analyze_chroma(
    source: &[u8],
    prediction: &[u8; 64],
    reconstruction: &mut [u8],
    stride: usize,
    macroblock: (usize, usize),
    quantization: QuantizationFactors,
    levels: &mut [[i16; 16]; 4],
) {
    let (macroblock_x, macroblock_y) = macroblock;
    for (block, block_levels) in levels.iter_mut().enumerate() {
        let block_x = block % 2;
        let block_y = block / 2;
        let samples = residual_block(
            source,
            stride,
            (
                macroblock_x * 8 + block_x * 4,
                macroblock_y * 8 + block_y * 4,
            ),
            prediction,
            8,
            (block_x * 4, block_y * 4),
        );
        *block_levels = quantize_block(
            &forward_dct(&samples),
            quantization.chroma_dc,
            quantization.chroma_ac,
        );
        let coefficients =
            dequantize_block(block_levels, quantization.chroma_dc, quantization.chroma_ac);
        write_reconstruction_block(
            reconstruction,
            stride,
            (
                macroblock_x * 8 + block_x * 4,
                macroblock_y * 8 + block_y * 4,
            ),
            prediction,
            8,
            (block_x * 4, block_y * 4),
            &inverse_dct(&coefficients),
        );
    }
}

fn residual_block(
    source: &[u8],
    source_stride: usize,
    source_position: (usize, usize),
    prediction: &[u8],
    prediction_stride: usize,
    prediction_position: (usize, usize),
) -> [i32; 16] {
    let (source_x, source_y) = source_position;
    let (prediction_x, prediction_y) = prediction_position;
    let mut block = [0; 16];
    for row in 0..4 {
        for column in 0..4 {
            block[row * 4 + column] =
                i32::from(source[(source_y + row) * source_stride + source_x + column])
                    - i32::from(
                        prediction
                            [(prediction_y + row) * prediction_stride + prediction_x + column],
                    );
        }
    }
    block
}

fn write_reconstruction_block(
    reconstruction: &mut [u8],
    reconstruction_stride: usize,
    reconstruction_position: (usize, usize),
    prediction: &[u8],
    prediction_stride: usize,
    prediction_position: (usize, usize),
    residual: &[i32; 16],
) {
    let (reconstruction_x, reconstruction_y) = reconstruction_position;
    let (prediction_x, prediction_y) = prediction_position;
    for row in 0..4 {
        for column in 0..4 {
            reconstruction
                [(reconstruction_y + row) * reconstruction_stride + reconstruction_x + column] =
                clamped_add(
                    prediction[(prediction_y + row) * prediction_stride + prediction_x + column],
                    residual[row * 4 + column],
                );
        }
    }
}

fn assemble_vp8(
    width: usize,
    height: usize,
    first_partition: &[u8],
    second_partition: &[u8],
) -> Vec<u8> {
    let mut vp8 = Vec::with_capacity(10 + first_partition.len() + second_partition.len());
    let frame_tag = 0x10 | ((first_partition.len() as u32) << 5);
    vp8.extend_from_slice(&frame_tag.to_le_bytes()[..3]);
    vp8.extend_from_slice(&[0x9d, 0x01, 0x2a]);
    vp8.extend_from_slice(&(width as u16).to_le_bytes());
    vp8.extend_from_slice(&(height as u16).to_le_bytes());
    vp8.extend_from_slice(first_partition);
    vp8.extend_from_slice(second_partition);
    vp8
}

fn assemble_riff(
    vp8: &[u8],
    pixels: &[u8],
    width: usize,
    height: usize,
    bytes_per_pixel: usize,
    options: &Options,
) -> Vec<u8> {
    let has_alpha = bytes_per_pixel == 4
        && options.alpha == Alpha::Lossless
        && pixels[3..].iter().step_by(4).any(|value| *value != 255);
    let extended = has_alpha || options.force_vp8x;
    let vp8_padding = vp8.len() & 1;
    let alpha_size = usize::from(has_alpha) * (1 + width * height);
    let alpha_padding = alpha_size & 1;
    let extended_size = usize::from(extended) * 18;
    let alpha_chunk_size = usize::from(has_alpha) * (8 + alpha_size + alpha_padding);
    let mut output =
        Vec::with_capacity(20 + vp8.len() + vp8_padding + extended_size + alpha_chunk_size);
    output.extend_from_slice(b"RIFF");
    output.extend_from_slice(&[0; 4]);
    output.extend_from_slice(b"WEBP");

    if extended {
        output.extend_from_slice(b"VP8X");
        output.extend_from_slice(&10u32.to_le_bytes());
        output.push(if has_alpha { 0x10 } else { 0x00 });
        output.extend_from_slice(&[0; 3]);
        output.extend_from_slice(&(width as u32 - 1).to_le_bytes()[..3]);
        output.extend_from_slice(&(height as u32 - 1).to_le_bytes()[..3]);
    }

    if has_alpha {
        output.extend_from_slice(b"ALPH");
        output.extend_from_slice(&(alpha_size as u32).to_le_bytes());
        // The WebP Container Specification's Alpha section assigns zero to
        // method 0 compression, filtering, preprocessing, and reserved bits.
        output.push(0);
        output.extend(pixels[3..].iter().step_by(4).copied());
        if alpha_padding != 0 {
            output.push(0);
        }
    }

    output.extend_from_slice(b"VP8 ");
    output.extend_from_slice(&(vp8.len() as u32).to_le_bytes());
    output.extend_from_slice(vp8);
    if vp8_padding != 0 {
        output.push(0);
    }
    let riff_size = (output.len() - 8) as u32;
    output[4..8].copy_from_slice(&riff_size.to_le_bytes());
    output
}

#[cfg(test)]
mod tests {
    use super::{
        encode, write_frame_header, DC_MODE, KF_UV_MODE_PROBS, KF_Y_MODE_PROBS, KF_Y_MODE_TREE,
        UV_MODE_TREE,
    };
    use crate::bool_coder::BoolEncoder;
    use crate::generator;
    use crate::quantize::{factors, quantize_block, quantizer_index, Q_TO_INDEX};
    use crate::transform::{forward_dct, forward_wht};
    use crate::{Alpha, Filter, Options};
    use std::format;
    use std::fs;
    use std::io::Cursor;
    use std::path::{Path, PathBuf};
    use std::process::Command;
    use std::string::ToString;
    use std::vec;
    use std::vec::Vec;

    fn path(tree: &[i8], probabilities: &[u8], value: u8) -> alloc::vec::Vec<(u8, bool)> {
        struct Trace(alloc::vec::Vec<(u8, bool)>);

        let mut encoder = BoolEncoder::new();
        encoder.write_tree(tree, probabilities, value, 0);
        let bytes = encoder.finish();
        let mut decoder = crate::bool_coder::BoolDecoder::new(&bytes);
        let mut node = 0usize;
        let mut trace = Trace(alloc::vec::Vec::new());
        loop {
            let probability = probabilities[node >> 1];
            let branch = decoder.read_bool(probability);
            trace.0.push((probability, branch));
            let child = tree[node + usize::from(branch)];
            if child <= 0 {
                assert_eq!(child.unsigned_abs(), value);
                return trace.0;
            }
            node = child as usize;
        }
    }

    #[test]
    fn the_mode_tables_match_the_section_11_pins_and_paths() {
        assert_eq!(KF_Y_MODE_PROBS.len(), 4);
        assert_eq!(KF_Y_MODE_PROBS.first(), Some(&145));
        assert_eq!(KF_Y_MODE_PROBS.last(), Some(&128));
        assert_eq!(
            KF_Y_MODE_PROBS
                .iter()
                .map(|value| u16::from(*value))
                .sum::<u16>(),
            592
        );
        assert_eq!(KF_UV_MODE_PROBS.len(), 3);
        assert_eq!(KF_UV_MODE_PROBS.first(), Some(&142));
        assert_eq!(KF_UV_MODE_PROBS.last(), Some(&183));
        assert_eq!(
            KF_UV_MODE_PROBS
                .iter()
                .map(|value| u16::from(*value))
                .sum::<u16>(),
            439
        );
        assert_eq!(
            path(&KF_Y_MODE_TREE, &KF_Y_MODE_PROBS, 0),
            [(145, true), (156, false), (163, false)]
        );
        assert_eq!(path(&UV_MODE_TREE, &KF_UV_MODE_PROBS, 0), [(142, false)]);
    }

    #[test]
    fn an_opaque_file_carries_one_padded_vp8_chunk_with_the_frame_fields() {
        let pixels = [
            0, 32, 64, 96, 128, 160, 192, 224, 255, 17, 34, 51, 68, 85, 102, 119, 136, 153,
        ];
        let options = Options {
            alpha: Alpha::Discard,
            filter: Filter::Off,
            ..Options::default()
        };
        let encoded = encode(&pixels, 3, 2, 3, 38, &options).webp;
        let riff_size = u32::from_le_bytes(encoded[4..8].try_into().expect("RIFF size bytes"));
        let chunk_size =
            u32::from_le_bytes(encoded[16..20].try_into().expect("chunk size bytes")) as usize;
        let payload = &encoded[20..20 + chunk_size];
        let frame_tag =
            u32::from(payload[0]) | (u32::from(payload[1]) << 8) | (u32::from(payload[2]) << 16);
        let first_partition_size = (frame_tag >> 5) as usize;

        assert_eq!(&encoded[..4], b"RIFF");
        assert_eq!(riff_size as usize, encoded.len() - 8);
        assert_eq!(&encoded[8..12], b"WEBP");
        assert_eq!(&encoded[12..16], b"VP8 ");
        assert_eq!(encoded.len(), 20 + chunk_size + (chunk_size & 1));
        assert_eq!(encoded.get(20 + chunk_size), Some(&0));
        assert_eq!(frame_tag & 1, 0);
        assert_eq!((frame_tag >> 1) & 7, 0);
        assert_eq!((frame_tag >> 4) & 1, 1);
        assert_eq!(first_partition_size, 7);
        assert_eq!(&payload[3..6], &[0x9d, 0x01, 0x2a]);
        assert_eq!(u16::from_le_bytes([payload[6], payload[7]]), 3);
        assert_eq!(u16::from_le_bytes([payload[8], payload[9]]), 2);

        let mut header = crate::bool_coder::BoolDecoder::new(&payload[10..]);
        assert_eq!(header.read_literal(1), 0);
        assert_eq!(header.read_literal(1), 0);
        assert_eq!(header.read_literal(1), 0);
        assert_eq!(header.read_literal(1), 0);
        assert_eq!(header.read_literal(6), 0);
        assert_eq!(header.read_literal(3), 0);
        assert_eq!(header.read_literal(1), 0);
        assert_eq!(header.read_literal(2), 0);
        assert_eq!(header.read_literal(7), 38);
        for _ in 0..5 {
            assert_eq!(header.read_literal(1), 0);
        }
        assert_eq!(header.read_literal(1), 1);
        for probability in crate::residual::COEFF_UPDATE_PROBS {
            assert_eq!(u8::from(header.read_bool(probability)), 0);
        }
        assert_eq!(header.read_literal(1), 0);
        assert_eq!(header.read_tree(&KF_Y_MODE_TREE, &KF_Y_MODE_PROBS, 0), 0);
        assert_eq!(header.read_tree(&UV_MODE_TREE, &KF_UV_MODE_PROBS, 0), 0);
    }

    #[test]
    fn filter_values_saturate_and_auto_matches_off_at_version_zero_point_one() {
        let pixels = [128u8; 3];
        let saturated_options = Options {
            filter: Filter::Level {
                level: 255,
                sharpness: 255,
            },
            ..Options::default()
        };
        let saturated = encode(&pixels, 1, 1, 3, 26, &saturated_options).webp;
        let mut header = crate::bool_coder::BoolDecoder::new(&saturated[30..]);
        assert_eq!(header.read_literal(1), 0);
        assert_eq!(header.read_literal(1), 0);
        assert_eq!(header.read_literal(1), 0);
        assert_eq!(header.read_literal(1), 0);
        assert_eq!(header.read_literal(6), 63);
        assert_eq!(header.read_literal(3), 7);

        let auto = encode(&pixels, 1, 1, 3, 26, &Options::default()).webp;
        let off_options = Options {
            filter: Filter::Off,
            ..Options::default()
        };
        let off = encode(&pixels, 1, 1, 3, 26, &off_options).webp;
        assert_eq!(auto, off);
    }

    #[test]
    fn a_nonopaque_image_carries_vp8x_alph_and_vp8_chunks_in_order() {
        let pixels = [10, 20, 30, 7, 40, 50, 60, 255];
        let encoded = encode(&pixels, 2, 1, 4, 26, &Options::default()).webp;

        assert_eq!(&encoded[..4], b"RIFF");
        assert_eq!(
            u32::from_le_bytes(encoded[4..8].try_into().expect("RIFF size bytes")) as usize,
            encoded.len() - 8
        );
        assert_eq!(&encoded[8..12], b"WEBP");
        assert_eq!(&encoded[12..16], b"VP8X");
        assert_eq!(&encoded[16..20], &10u32.to_le_bytes());
        assert_eq!(&encoded[20..30], &[0x10, 0, 0, 0, 1, 0, 0, 0, 0, 0]);
        assert_eq!(&encoded[30..34], b"ALPH");
        assert_eq!(&encoded[34..38], &3u32.to_le_bytes());
        assert_eq!(&encoded[38..41], &[0, 7, 255]);
        assert_eq!(encoded[41], 0);
        assert_eq!(&encoded[42..46], b"VP8 ");
    }

    #[test]
    fn forcing_vp8x_on_opaque_inputs_sets_no_alpha_flag_or_chunk() {
        let rgba = [10, 20, 30, 255];
        let rgb = [10, 20, 30];
        let options = Options {
            force_vp8x: true,
            filter: Filter::Off,
            ..Options::default()
        };
        let from_rgba =
            crate::encode_rgba(&rgba, 1, 1, &options).expect("encode the forced RGBA container");
        let from_rgb =
            crate::encode_rgb(&rgb, 1, 1, &options).expect("encode the forced RGB container");

        assert_eq!(from_rgba, from_rgb);
        assert_eq!(&from_rgba[12..16], b"VP8X");
        assert_eq!(&from_rgba[16..20], &10u32.to_le_bytes());
        assert_eq!(&from_rgba[20..30], &[0; 10]);
        assert_eq!(&from_rgba[30..34], b"VP8 ");
    }

    #[test]
    fn the_largest_frame_keeps_its_first_partition_inside_nineteen_bits() {
        let macroblock_count = 1024 * 1024;
        let mut partition = BoolEncoder::with_capacity(524_288);
        write_frame_header(&mut partition, 0, Filter::Off);
        for _ in 0..macroblock_count {
            partition.write_tree(&KF_Y_MODE_TREE, &KF_Y_MODE_PROBS, DC_MODE, 0);
            partition.write_tree(&UV_MODE_TREE, &KF_UV_MODE_PROBS, DC_MODE, 0);
        }
        let partition_size = partition.finish().len();
        assert_eq!(partition_size, 449_397);
        assert_eq!(partition_size >> 19, 0);
    }

    #[test]
    fn extreme_pipeline_residuals_keep_every_quantized_level_below_2047() {
        let quantization = factors(0);
        let mut seed = 0x8f61_32d9u32;
        let mut largest = 0u16;
        for _ in 0..4096 {
            let mut dc = [0; 16];
            for dc_value in &mut dc {
                let mut block = [0; 16];
                for value in &mut block {
                    seed ^= seed << 13;
                    seed ^= seed >> 17;
                    seed ^= seed << 5;
                    *value = if seed & 1 == 0 { -255 } else { 255 };
                }
                let coefficients = forward_dct(&block);
                *dc_value = coefficients[0];
                let levels = quantize_block(&coefficients, quantization.y_dc, quantization.y_ac);
                largest = largest.max(
                    levels
                        .iter()
                        .map(|level| level.unsigned_abs())
                        .max()
                        .unwrap(),
                );
            }
            let y2 = quantize_block(&forward_wht(&dc), quantization.y2_dc, quantization.y2_ac);
            largest = largest.max(y2.iter().map(|level| level.unsigned_abs()).max().unwrap());
        }
        assert_eq!(largest, 606);
    }

    #[test]
    fn every_fixture_decodes_at_its_dimensions_with_exact_alpha_at_each_quality() {
        for fixture in generator::all() {
            let expected_alpha = alpha_bytes(&fixture.rgba);
            let has_alpha = expected_alpha.iter().any(|value| *value != 255);
            for quality in [0u8, 25, 50, 75, 90, 95, 100] {
                let options = Options {
                    quality,
                    ..Options::default()
                };
                let encoded =
                    crate::encode_rgba(&fixture.rgba, fixture.width, fixture.height, &options)
                        .expect("encode the fixture");
                let mut decoder = image_webp::WebPDecoder::new(Cursor::new(encoded))
                    .expect("decode the WebP header");
                assert_eq!(decoder.dimensions(), (fixture.width, fixture.height));
                assert_eq!(decoder.has_alpha(), has_alpha);
                let channels = if has_alpha { 4 } else { 3 };
                let mut pixels =
                    vec![0; fixture.width as usize * fixture.height as usize * channels];
                assert_eq!(u8::from(decoder.read_image(&mut pixels).is_ok()), 1);
                if has_alpha {
                    assert_eq!(
                        alpha_bytes(&pixels),
                        expected_alpha,
                        "{} q{quality}",
                        fixture.name
                    );
                }
            }
        }
    }

    #[test]
    fn opaque_rgba_and_rgb_inputs_use_a_bare_vp8_chunk_at_each_quality() {
        let fixtures = generator::all();
        for fixture in &fixtures {
            let rgb = rgb_bytes(&fixture.rgba);
            for quality in [0u8, 25, 50, 75, 90, 95, 100] {
                let options = Options {
                    quality,
                    ..Options::default()
                };
                let from_rgb = crate::encode_rgb(&rgb, fixture.width, fixture.height, &options)
                    .expect("encode the RGB fixture");
                assert_eq!(&from_rgb[12..16], b"VP8 ", "{} q{quality}", fixture.name);
            }
        }

        let opaque = fixtures
            .iter()
            .find(|fixture| fixture.name == "flat")
            .expect("find the opaque fixture");
        for quality in [0u8, 25, 50, 75, 90, 95, 100] {
            let options = Options {
                quality,
                ..Options::default()
            };
            let encoded = crate::encode_rgba(&opaque.rgba, opaque.width, opaque.height, &options)
                .expect("encode the opaque RGBA fixture");
            assert_eq!(&encoded[12..16], b"VP8 ", "{} q{quality}", opaque.name);
        }
    }

    #[test]
    fn discarded_alpha_matches_rgb_input_bytes_at_each_quality() {
        for fixture in generator::all()
            .into_iter()
            .filter(|fixture| has_nonopaque_alpha(&fixture.rgba))
        {
            let rgb = rgb_bytes(&fixture.rgba);
            for quality in [0u8, 25, 50, 75, 90, 95, 100] {
                let mut options = Options {
                    quality,
                    ..Options::default()
                };
                options.alpha = Alpha::Discard;
                let from_rgba =
                    crate::encode_rgba(&fixture.rgba, fixture.width, fixture.height, &options)
                        .expect("encode the RGBA fixture");
                let from_rgb = crate::encode_rgb(&rgb, fixture.width, fixture.height, &options)
                    .expect("encode the RGB fixture");
                assert_eq!(from_rgba, from_rgb, "{} q{quality}", fixture.name);
                assert_eq!(&from_rgba[12..16], b"VP8 ", "{} q{quality}", fixture.name);
            }
        }
    }

    #[test]
    fn dwebp_yuv_output_ends_with_each_input_alpha_plane_at_each_quality() {
        if !oracle_is_available("dwebp") {
            return;
        }
        let directory = scratch_directory("dwebp_preserves_alpha");
        let input = directory.join("input.webp");
        let output = directory.join("output.yuv");
        for fixture in generator::all()
            .into_iter()
            .filter(|fixture| has_nonopaque_alpha(&fixture.rgba))
        {
            let expected_alpha = alpha_bytes(&fixture.rgba);
            for quality in [0u8, 25, 50, 75, 90, 95, 100] {
                let options = Options {
                    quality,
                    ..Options::default()
                };
                let encoded =
                    crate::encode_rgba(&fixture.rgba, fixture.width, fixture.height, &options)
                        .expect("encode the alpha fixture");
                fs::write(&input, encoded).expect("write the WebP file");
                let status = Command::new("dwebp")
                    .arg("-quiet")
                    .arg("-yuv")
                    .arg(&input)
                    .arg("-o")
                    .arg(&output)
                    .status()
                    .expect("run dwebp");
                assert_eq!(u8::from(status.success()), 1, "{} q{quality}", fixture.name);
                let decoded = fs::read(&output).expect("read the decoded YUV planes");
                let chroma_pixels =
                    fixture.width.div_ceil(2) as usize * fixture.height.div_ceil(2) as usize;
                let alpha_start =
                    fixture.width as usize * fixture.height as usize + 2 * chroma_pixels;
                assert_eq!(decoded.len(), alpha_start + expected_alpha.len());
                assert_eq!(
                    &decoded[alpha_start..],
                    expected_alpha,
                    "{} q{quality}",
                    fixture.name
                );
            }
        }
        fs::remove_dir_all(directory).expect("remove the test directory");
    }

    #[test]
    fn dwebp_decodes_every_fixture_at_its_dimensions_at_each_quality() {
        if !oracle_is_available("dwebp") {
            return;
        }
        let directory = scratch_directory("dwebp_decodes_every_fixture");
        let input = directory.join("input.webp");
        let output = directory.join("output.png");
        for fixture in generator::all() {
            for quality in [0u8, 25, 50, 75, 90, 95, 100] {
                let options = Options {
                    quality,
                    ..Options::default()
                };
                let encoded =
                    crate::encode_rgba(&fixture.rgba, fixture.width, fixture.height, &options)
                        .expect("encode the fixture");
                fs::write(&input, encoded).expect("write the WebP file");
                let status = Command::new("dwebp")
                    .arg("-quiet")
                    .arg(&input)
                    .arg("-o")
                    .arg(&output)
                    .status()
                    .expect("run dwebp");
                assert_eq!(u8::from(status.success()), 1, "{} q{quality}", fixture.name);
                assert_eq!(png_dimensions(&output), (fixture.width, fixture.height));
            }
        }
        fs::remove_dir_all(directory).expect("remove the test directory");
    }

    #[test]
    fn reconstruction_matches_dwebp_for_every_fixture_quality_and_index() {
        if !oracle_is_available("dwebp") {
            return;
        }
        let directory = scratch_directory("reconstruction_matches_dwebp");
        for fixture in generator::all() {
            for quality in [0u8, 25, 50, 75, 90, 95, 100] {
                assert_reconstruction(&directory, &fixture, quantizer_index(quality));
            }
        }
        for name in ["gradient", "noise"] {
            let fixtures = generator::all();
            let fixture = fixtures
                .iter()
                .find(|fixture| fixture.name == name)
                .expect("find the indexed fixture");
            for index in 0..=127u8 {
                assert_reconstruction(&directory, fixture, index);
            }
        }
        fs::remove_dir_all(directory).expect("remove the test directory");
    }

    #[test]
    fn a_reconstruction_difference_names_its_fixture_index_plane_row_and_column() {
        assert_eq!(
            reconstruction_difference_message("flat", 26, "U", 3, 7),
            "fixture flat, quantizer index 26, plane U, first differing row 3, column 7"
        );
    }

    #[test]
    fn cwebp_writes_each_checked_in_quality_index_into_its_frame_header() {
        if !oracle_is_available("cwebp") {
            return;
        }
        let directory = scratch_directory("cwebp_quality_indices");
        let fixtures = generator::all();
        let fixture = fixtures
            .iter()
            .find(|fixture| fixture.name == "gradient")
            .expect("find the gradient fixture");
        write_png(&directory.join("gradient.png"), fixture);
        let input = directory.join("gradient.png");
        let output = directory.join("output.webp");
        for (quality, expected) in Q_TO_INDEX.iter().enumerate() {
            let status = Command::new("cwebp")
                .arg("-quiet")
                .arg("-q")
                .arg(quality.to_string())
                .arg("-segments")
                .arg("1")
                .arg("-sns")
                .arg("0")
                .arg(&input)
                .arg("-o")
                .arg(&output)
                .status()
                .expect("run cwebp");
            assert_eq!(u8::from(status.success()), 1, "quality {quality}");
            let webp = fs::read(&output).expect("read the cwebp output");
            assert_eq!(read_quantizer_index(&webp), *expected);
        }
        fs::remove_dir_all(directory).expect("remove the test directory");
    }

    fn alpha_bytes(rgba: &[u8]) -> Vec<u8> {
        rgba[3..].iter().step_by(4).copied().collect()
    }

    fn rgb_bytes(rgba: &[u8]) -> Vec<u8> {
        rgba.chunks_exact(4)
            .flat_map(|pixel| pixel[..3].iter().copied())
            .collect()
    }

    fn has_nonopaque_alpha(rgba: &[u8]) -> bool {
        rgba[3..].iter().step_by(4).any(|value| *value != 255)
    }

    fn oracle_is_available(command: &str) -> bool {
        let available = Command::new(command)
            .arg("-version")
            .output()
            .map(|output| output.status.success())
            .unwrap_or(false);
        if !available {
            let required = std::env::var("TINY_WEBP_REQUIRE_ORACLE").as_deref() == Ok("1");
            assert_eq!(u8::from(required), 0, "{command} is required");
        }
        available
    }

    fn scratch_directory(test_name: &str) -> PathBuf {
        let directory =
            std::env::temp_dir().join(format!("tiny-webp-{test_name}-{}", std::process::id()));
        if directory.exists() {
            fs::remove_dir_all(&directory).expect("remove the old test directory");
        }
        fs::create_dir_all(&directory).expect("create the test directory");
        directory
    }

    fn png_dimensions(path: &Path) -> (u32, u32) {
        let bytes = fs::read(path).expect("read the decoded PNG header");
        assert_eq!(&bytes[..8], b"\x89PNG\r\n\x1a\n");
        assert_eq!(&bytes[12..16], b"IHDR");
        let width = u32::from_be_bytes(bytes[16..20].try_into().expect("PNG width bytes"));
        let height = u32::from_be_bytes(bytes[20..24].try_into().expect("PNG height bytes"));
        (width, height)
    }

    fn write_png(path: &Path, fixture: &generator::Fixture) {
        let output = std::io::BufWriter::new(fs::File::create(path).expect("create the PNG file"));
        let mut encoder = png::Encoder::new(output, fixture.width, fixture.height);
        encoder.set_color(png::ColorType::Rgba);
        encoder.set_depth(png::BitDepth::Eight);
        let mut writer = encoder.write_header().expect("write the PNG header");
        writer
            .write_image_data(&fixture.rgba)
            .expect("write the PNG pixels");
    }

    fn assert_reconstruction(directory: &Path, fixture: &generator::Fixture, index: u8) {
        let options = Options {
            filter: Filter::Off,
            ..Options::default()
        };
        let encoded = encode(
            &fixture.rgba,
            fixture.width as usize,
            fixture.height as usize,
            4,
            index,
            &options,
        );
        let input = directory.join("input.webp");
        let output = directory.join("output.yuv");
        fs::write(&input, &encoded.webp).expect("write the WebP file");
        let status = Command::new("dwebp")
            .arg("-quiet")
            .arg("-yuv")
            .arg(&input)
            .arg("-o")
            .arg(&output)
            .status()
            .expect("run dwebp");
        assert_eq!(
            u8::from(status.success()),
            1,
            "fixture {}, quantizer index {index}",
            fixture.name
        );
        let decoded = fs::read(output).expect("read the decoded YUV planes");
        let width = fixture.width as usize;
        let height = fixture.height as usize;
        let chroma_width = (fixture.width as usize).div_ceil(2);
        let chroma_height = (fixture.height as usize).div_ceil(2);
        let y_length = width * height;
        let chroma_length = chroma_width * chroma_height;
        assert_plane_matches(
            fixture,
            index,
            "Y",
            &encoded.reconstruction.y,
            encoded.reconstruction.y_stride,
            &decoded[..y_length],
            (width, height),
        );
        assert_plane_matches(
            fixture,
            index,
            "U",
            &encoded.reconstruction.u,
            encoded.reconstruction.chroma_stride,
            &decoded[y_length..y_length + chroma_length],
            (chroma_width, chroma_height),
        );
        assert_plane_matches(
            fixture,
            index,
            "V",
            &encoded.reconstruction.v,
            encoded.reconstruction.chroma_stride,
            &decoded[y_length + chroma_length..y_length + 2 * chroma_length],
            (chroma_width, chroma_height),
        );
        if has_nonopaque_alpha(&fixture.rgba) {
            let alpha = alpha_bytes(&fixture.rgba);
            assert_plane_matches(
                fixture,
                index,
                "A",
                &alpha,
                width,
                &decoded[y_length + 2 * chroma_length..],
                (width, height),
            );
        }
    }

    fn assert_plane_matches(
        fixture: &generator::Fixture,
        index: u8,
        plane: &str,
        reconstruction: &[u8],
        reconstruction_stride: usize,
        decoded: &[u8],
        dimensions: (usize, usize),
    ) {
        let (width, height) = dimensions;
        for row in 0..height {
            for column in 0..width {
                let decoded_value = decoded[row * width + column];
                let reconstruction_value = reconstruction[row * reconstruction_stride + column];
                if decoded_value != reconstruction_value {
                    let message =
                        reconstruction_difference_message(fixture.name, index, plane, row, column);
                    assert_eq!(decoded_value, reconstruction_value, "{message}");
                }
            }
        }
    }

    fn reconstruction_difference_message(
        fixture: &str,
        index: u8,
        plane: &str,
        row: usize,
        column: usize,
    ) -> std::string::String {
        format!(
            "fixture {fixture}, quantizer index {index}, plane {plane}, first differing row {row}, column {column}"
        )
    }

    fn read_quantizer_index(webp: &[u8]) -> u8 {
        assert_eq!(&webp[..4], b"RIFF");
        assert_eq!(&webp[8..12], b"WEBP");
        assert_eq!(&webp[12..16], b"VP8 ");
        let payload = &webp[20..];
        let mut decoder = crate::bool_coder::BoolDecoder::new(&payload[10..]);
        assert_eq!(decoder.read_literal(1), 0);
        assert_eq!(decoder.read_literal(1), 0);
        let segmentation_enabled = decoder.read_bool(128);
        if segmentation_enabled {
            let update_map = decoder.read_bool(128);
            let update_data = decoder.read_bool(128);
            if update_data {
                decoder.read_literal(1);
                for width in [7u8; 4].into_iter().chain([6u8; 4]) {
                    if decoder.read_bool(128) {
                        decoder.read_literal(width);
                        decoder.read_literal(1);
                    }
                }
            }
            if update_map {
                for _ in 0..3 {
                    if decoder.read_bool(128) {
                        decoder.read_literal(8);
                    }
                }
            }
        }
        decoder.read_literal(1);
        decoder.read_literal(6);
        decoder.read_literal(3);
        if decoder.read_bool(128) && decoder.read_bool(128) {
            for _ in 0..8 {
                if decoder.read_bool(128) {
                    decoder.read_literal(6);
                    decoder.read_literal(1);
                }
            }
        }
        decoder.read_literal(2);
        decoder.read_literal(7) as u8
    }
}
