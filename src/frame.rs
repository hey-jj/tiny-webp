use alloc::vec;
use alloc::vec::Vec;

use crate::bool_coder::BoolEncoder;
use crate::color::{convert, YuvPlanes};
use crate::prediction::{predict_chroma, predict_luma, CANDIDATES};
use crate::quantize::{
    dequantize_block, factors, quantize_block, QuantizationFactors, QUANTIZER_DELTAS,
};
use crate::residual::{
    BitEstimate, EntropyWriter, MacroblockResidual, ProbabilityWriter, ResidualWriter,
    TokenMacroblock, TokenStatistics, TokenStream, COEFF_UPDATE_PROBS, DEFAULT_COEFF_PROBS,
};
use crate::transform::{clamped_add, forward_dct, forward_wht, inverse_dct, inverse_wht};
use crate::{Alpha, Filter, Options};

const SUBBLOCK_MODE: u8 = 4;

// RFC 6386 section 11.2 defines the key-frame luma mode tree and probabilities.
pub(crate) const KF_Y_MODE_TREE: [i8; 8] = [-4, 2, 4, 6, 0, -1, -2, -3];
pub(crate) const KF_Y_MODE_PROBS: [u8; 4] = [145, 156, 163, 128];

// RFC 6386 section 11.4 defines the chroma mode tree and probabilities.
pub(crate) const UV_MODE_TREE: [i8; 6] = [0, 2, -1, 4, -2, -3];
pub(crate) const KF_UV_MODE_PROBS: [u8; 3] = [142, 114, 183];

// RFC 6386 section 11.2 fixes the sub-block tree leaf numbering.
const B_MODE_TREE: [i8; 18] = [
    0, 2, -1, 4, -2, 6, 8, 12, -3, 10, -5, -6, -4, 14, -7, 16, -8, -9,
];

// RFC 6386 section 11.5 indexes these probabilities by above and left modes.
const KF_B_MODE_PROBS: [[[u8; 9]; 10]; 10] = [
    [
        [231, 120, 48, 89, 115, 113, 120, 152, 112],
        [152, 179, 64, 126, 170, 118, 46, 70, 95],
        [175, 69, 143, 80, 85, 82, 72, 155, 103],
        [56, 58, 10, 171, 218, 189, 17, 13, 152],
        [144, 71, 10, 38, 171, 213, 144, 34, 26],
        [114, 26, 17, 163, 44, 195, 21, 10, 173],
        [121, 24, 80, 195, 26, 62, 44, 64, 85],
        [170, 46, 55, 19, 136, 160, 33, 206, 71],
        [63, 20, 8, 114, 114, 208, 12, 9, 226],
        [81, 40, 11, 96, 182, 84, 29, 16, 36],
    ],
    [
        [134, 183, 89, 137, 98, 101, 106, 165, 148],
        [72, 187, 100, 130, 157, 111, 32, 75, 80],
        [66, 102, 167, 99, 74, 62, 40, 234, 128],
        [41, 53, 9, 178, 241, 141, 26, 8, 107],
        [104, 79, 12, 27, 217, 255, 87, 17, 7],
        [74, 43, 26, 146, 73, 166, 49, 23, 157],
        [65, 38, 105, 160, 51, 52, 31, 115, 128],
        [87, 68, 71, 44, 114, 51, 15, 186, 23],
        [47, 41, 14, 110, 182, 183, 21, 17, 194],
        [66, 45, 25, 102, 197, 189, 23, 18, 22],
    ],
    [
        [88, 88, 147, 150, 42, 46, 45, 196, 205],
        [43, 97, 183, 117, 85, 38, 35, 179, 61],
        [39, 53, 200, 87, 26, 21, 43, 232, 171],
        [56, 34, 51, 104, 114, 102, 29, 93, 77],
        [107, 54, 32, 26, 51, 1, 81, 43, 31],
        [39, 28, 85, 171, 58, 165, 90, 98, 64],
        [34, 22, 116, 206, 23, 34, 43, 166, 73],
        [68, 25, 106, 22, 64, 171, 36, 225, 114],
        [34, 19, 21, 102, 132, 188, 16, 76, 124],
        [62, 18, 78, 95, 85, 57, 50, 48, 51],
    ],
    [
        [193, 101, 35, 159, 215, 111, 89, 46, 111],
        [60, 148, 31, 172, 219, 228, 21, 18, 111],
        [112, 113, 77, 85, 179, 255, 38, 120, 114],
        [40, 42, 1, 196, 245, 209, 10, 25, 109],
        [100, 80, 8, 43, 154, 1, 51, 26, 71],
        [88, 43, 29, 140, 166, 213, 37, 43, 154],
        [61, 63, 30, 155, 67, 45, 68, 1, 209],
        [142, 78, 78, 16, 255, 128, 34, 197, 171],
        [41, 40, 5, 102, 211, 183, 4, 1, 221],
        [51, 50, 17, 168, 209, 192, 23, 25, 82],
    ],
    [
        [125, 98, 42, 88, 104, 85, 117, 175, 82],
        [95, 84, 53, 89, 128, 100, 113, 101, 45],
        [75, 79, 123, 47, 51, 128, 81, 171, 1],
        [57, 17, 5, 71, 102, 57, 53, 41, 49],
        [115, 21, 2, 10, 102, 255, 166, 23, 6],
        [38, 33, 13, 121, 57, 73, 26, 1, 85],
        [41, 10, 67, 138, 77, 110, 90, 47, 114],
        [101, 29, 16, 10, 85, 128, 101, 196, 26],
        [57, 18, 10, 102, 102, 213, 34, 20, 43],
        [117, 20, 15, 36, 163, 128, 68, 1, 26],
    ],
    [
        [138, 31, 36, 171, 27, 166, 38, 44, 229],
        [67, 87, 58, 169, 82, 115, 26, 59, 179],
        [63, 59, 90, 180, 59, 166, 93, 73, 154],
        [40, 40, 21, 116, 143, 209, 34, 39, 175],
        [57, 46, 22, 24, 128, 1, 54, 17, 37],
        [47, 15, 16, 183, 34, 223, 49, 45, 183],
        [46, 17, 33, 183, 6, 98, 15, 32, 183],
        [65, 32, 73, 115, 28, 128, 23, 128, 205],
        [40, 3, 9, 115, 51, 192, 18, 6, 223],
        [87, 37, 9, 115, 59, 77, 64, 21, 47],
    ],
    [
        [104, 55, 44, 218, 9, 54, 53, 130, 226],
        [64, 90, 70, 205, 40, 41, 23, 26, 57],
        [54, 57, 112, 184, 5, 41, 38, 166, 213],
        [30, 34, 26, 133, 152, 116, 10, 32, 134],
        [75, 32, 12, 51, 192, 255, 160, 43, 51],
        [39, 19, 53, 221, 26, 114, 32, 73, 255],
        [31, 9, 65, 234, 2, 15, 1, 118, 73],
        [88, 31, 35, 67, 102, 85, 55, 186, 85],
        [56, 21, 23, 111, 59, 205, 45, 37, 192],
        [55, 38, 70, 124, 73, 102, 1, 34, 98],
    ],
    [
        [102, 61, 71, 37, 34, 53, 31, 243, 192],
        [69, 60, 71, 38, 73, 119, 28, 222, 37],
        [68, 45, 128, 34, 1, 47, 11, 245, 171],
        [62, 17, 19, 70, 146, 85, 55, 62, 70],
        [75, 15, 9, 9, 64, 255, 184, 119, 16],
        [37, 43, 37, 154, 100, 163, 85, 160, 1],
        [63, 9, 92, 136, 28, 64, 32, 201, 85],
        [86, 6, 28, 5, 64, 255, 25, 248, 1],
        [56, 8, 17, 132, 137, 255, 55, 116, 128],
        [58, 15, 20, 82, 135, 57, 26, 121, 40],
    ],
    [
        [164, 50, 31, 137, 154, 133, 25, 35, 218],
        [51, 103, 44, 131, 131, 123, 31, 6, 158],
        [86, 40, 64, 135, 148, 224, 45, 183, 128],
        [22, 26, 17, 131, 240, 154, 14, 1, 209],
        [83, 12, 13, 54, 192, 255, 68, 47, 28],
        [45, 16, 21, 91, 64, 222, 7, 1, 197],
        [56, 21, 39, 155, 60, 138, 23, 102, 213],
        [85, 26, 85, 85, 128, 128, 32, 146, 171],
        [18, 11, 7, 63, 144, 171, 4, 4, 246],
        [35, 27, 10, 146, 174, 171, 12, 26, 128],
    ],
    [
        [190, 80, 35, 99, 180, 80, 126, 54, 45],
        [85, 126, 47, 87, 176, 51, 41, 20, 32],
        [101, 75, 128, 139, 118, 146, 116, 128, 85],
        [56, 41, 15, 176, 236, 85, 37, 9, 62],
        [146, 36, 19, 30, 171, 255, 97, 27, 20],
        [71, 30, 17, 119, 118, 255, 17, 18, 138],
        [101, 38, 60, 138, 55, 70, 43, 26, 142],
        [138, 45, 61, 62, 219, 1, 81, 188, 64],
        [32, 41, 20, 117, 151, 142, 20, 21, 163],
        [112, 19, 12, 61, 195, 128, 48, 4, 24],
    ],
];

pub(crate) struct EncodedFrame {
    pub(crate) webp: Vec<u8>,
    #[cfg(test)]
    pub(crate) reconstruction: YuvPlanes,
    #[cfg(test)]
    capacities: [(usize, usize); 3],
    #[cfg(test)]
    rung: u8,
}

pub(crate) fn encode(
    pixels: &[u8],
    width: usize,
    height: usize,
    bytes_per_pixel: usize,
    quantizer_index: u8,
    options: &Options,
) -> EncodedFrame {
    encode_with_decision(
        pixels,
        width,
        height,
        bytes_per_pixel,
        quantizer_index,
        options,
        ModeDecision::default(),
    )
}

fn encode_with_decision(
    pixels: &[u8],
    width: usize,
    height: usize,
    bytes_per_pixel: usize,
    quantizer_index: u8,
    options: &Options,
    decision: ModeDecision,
) -> EncodedFrame {
    #[cfg(test)]
    let partition_limit = decision.partition_limit;
    #[cfg(not(test))]
    let partition_limit = 524287;
    let mut decision = decision;
    let source = convert(pixels, width, height, bytes_per_pixel);
    let columns = width.div_ceil(16);
    let rows = height.div_ceil(16);
    let mut reconstruction = YuvPlanes {
        y: vec![0; source.y.len()],
        u: vec![0; source.u.len()],
        v: vec![0; source.v.len()],
        y_stride: source.y_stride,
        chroma_stride: source.chroma_stride,
    };
    let mut tokens =
        TokenStream::with_capacity(columns * rows * TokenStream::MAX_BYTES_PER_MACROBLOCK);
    let quantization = factors(quantizer_index);
    for rung in 0..3 {
        decision.whole_modes = if rung == 2 { 1 } else { 4 };
        decision.subblocks = rung == 0;
        tokens.clear();
        reconstruction.y.fill(0);
        reconstruction.u.fill(0);
        reconstruction.v.fill(0);
        let mut statistics = TokenStatistics::default();
        let mut contexts = ResidualWriter::new(columns);
        let mut unused = BitEstimate::default();
        #[cfg(test)]
        let token_capacity = tokens.capacity();
        for row in 0..rows {
            for column in 0..columns {
                let selected =
                    decision.analyze(&source, &mut reconstruction, (column, row), quantization);
                let has_y2 = selected.luma_mode != SUBBLOCK_MODE;
                statistics.record_macroblock(&selected.residual, has_y2);
                if rung != 2 && !selected.residual.has_coefficients(has_y2) {
                    contexts.skip_macroblock(column, has_y2);
                } else {
                    contexts.write_macroblock(
                        &mut statistics.writer(&mut unused),
                        column,
                        &selected.residual,
                        has_y2,
                    );
                }
                tokens.push(&selected);
            }
        }
        #[cfg(test)]
        let token_capacities = (token_capacity, tokens.capacity());
        let probabilities = statistics.probabilities();
        let skip = if rung == 2 {
            None
        } else {
            statistics.skip_probability()
        };
        let mut estimate = BitEstimate::default();
        write_frame_header(
            &mut estimate,
            quantizer_index,
            options.filter,
            &probabilities,
            skip,
        );
        write_modes(&mut estimate, &tokens, columns, skip);
        let first_capacity = estimate.0.div_ceil(8 * 256) as usize + 64;
        let second_capacity = statistics.estimated_bytes(&probabilities);
        let (output, chunk_start) = start_riff(
            pixels,
            width,
            height,
            bytes_per_pixel,
            options,
            first_capacity + second_capacity,
        );
        let frame_start = output.len() - 10;
        let first_start = output.len();
        let mut first = BoolEncoder::from_output(output);
        #[cfg(test)]
        let first_before = first.capacity();
        write_frame_header(
            &mut first,
            quantizer_index,
            options.filter,
            &probabilities,
            skip,
        );
        write_modes(&mut first, &tokens, columns, skip);
        let mut output = first.finish();
        #[cfg(test)]
        let first_capacities = (first_before, output.capacity());
        let first_size = output.len() - first_start;
        if first_size > partition_limit && rung < 2 {
            continue;
        }
        // RFC 6386 section 19.1 gives the first partition length field nineteen bits.
        assert!(
            first_size <= 524287,
            "the DC mode partition exceeds its field"
        );
        let tag = 0x10 | ((first_size as u32) << 5);
        output[frame_start..frame_start + 3].copy_from_slice(&tag.to_le_bytes()[..3]);
        drop(source);
        #[cfg(not(test))]
        drop(reconstruction);
        let mut second = BoolEncoder::from_output(output);
        #[cfg(test)]
        let second_before = second.capacity();
        let mut reader = tokens.reader();
        let mut contexts = ResidualWriter::new(columns);
        let mut position = 0;
        while let Some(block) = reader.next_macroblock() {
            let has_y2 = block.luma_mode != SUBBLOCK_MODE;
            let column = position % columns;
            if skip.is_some() && !block.residual.has_coefficients(has_y2) {
                contexts.skip_macroblock(column, has_y2);
            } else {
                contexts.write_macroblock(
                    &mut ProbabilityWriter {
                        encoder: &mut second,
                        probabilities: &probabilities,
                    },
                    column,
                    &block.residual,
                    has_y2,
                );
            }
            position += 1;
        }
        let mut output = second.finish();
        #[cfg(test)]
        let second_capacities = (second_before, output.capacity());
        let payload_size = output.len() - frame_start;
        output[chunk_start + 4..chunk_start + 8]
            .copy_from_slice(&(payload_size as u32).to_le_bytes());
        if payload_size & 1 != 0 {
            output.push(0);
        }
        let riff_size = (output.len() - 8) as u32;
        output[4..8].copy_from_slice(&riff_size.to_le_bytes());
        return EncodedFrame {
            webp: output,
            #[cfg(test)]
            reconstruction,
            #[cfg(test)]
            capacities: [token_capacities, first_capacities, second_capacities],
            #[cfg(test)]
            rung,
        };
    }
    unreachable!("the DC mode completes the frame")
}

fn write_modes<W: EntropyWriter>(
    encoder: &mut W,
    tokens: &TokenStream,
    columns: usize,
    skip: Option<u8>,
) {
    let mut writer = ModeWriter::new(columns);
    let mut reader = tokens.reader();
    let mut position = 0;
    while let Some(block) = reader.next_macroblock() {
        if let Some(probability) = skip {
            encoder.write_bool(
                probability,
                !block
                    .residual
                    .has_coefficients(block.luma_mode != SUBBLOCK_MODE),
            );
        }
        writer.write_macroblock(
            encoder,
            position % columns,
            block.luma_mode,
            block.chroma_mode,
            &block.subblock_modes,
        );
        position += 1;
    }
}

fn write_frame_header<W: EntropyWriter>(
    encoder: &mut W,
    quantizer_index: u8,
    filter: Filter,
    probabilities: &[u8; 1056],
    skip_probability: Option<u8>,
) {
    let (level, sharpness) = match filter {
        Filter::Level { level, sharpness } => (level.min(63), sharpness.min(7)),
        Filter::Auto => (quantizer_index.min(63), 0),
        Filter::Off => (0, 0),
    };

    // RFC 6386 section 9.2 reserves color_space 0 for YUV.
    encoder.write_literal(0, 1);
    // RFC 6386 section 9.2 requires decoder saturation with clamping_type 0.
    encoder.write_literal(0, 1);
    // RFC 6386 section 9.3 applies one quantizer when segmentation_enabled is 0.
    encoder.write_literal(0, 1);
    // RFC 6386 sections 9.4 and 15 define the normal filter selected by filter_type.
    encoder.write_literal(0, 1);
    // RFC 6386 section 9.4 limits loop_filter_level to six bits.
    encoder.write_literal(u32::from(level), 6);
    // RFC 6386 section 9.4 limits sharpness_level to three bits.
    encoder.write_literal(u32::from(sharpness), 3);
    // RFC 6386 section 9.4 keeps the frame level when loop_filter_adj_enable is 0.
    encoder.write_literal(0, 1);
    // RFC 6386 section 9.5 uses log2_nbr_of_dct_partitions 0 for one token partition.
    encoder.write_literal(0, 2);
    // RFC 6386 section 9.6 uses y_ac_qi as the base index for every plane.
    encoder.write_literal(u32::from(quantizer_index), 7);
    // RFC 6386 section 9.6 keeps the base Y DC index with y_dc_delta_present 0.
    encoder.write_literal(0, 1);
    // RFC 6386 section 9.6 keeps the base Y2 DC index with y2_dc_delta_present 0.
    encoder.write_literal(0, 1);
    // RFC 6386 section 9.6 keeps the base Y2 AC index with y2_ac_delta_present 0.
    encoder.write_literal(0, 1);
    // RFC 6386 section 9.6 encodes chroma DC delta -2 as magnitude and sign.
    encoder.write_literal(1, 1);
    encoder.write_literal(u32::from(QUANTIZER_DELTAS[3].unsigned_abs()), 4);
    encoder.write_literal(1, 1);
    // RFC 6386 section 9.6 encodes chroma AC delta -4 as magnitude and sign.
    encoder.write_literal(1, 1);
    encoder.write_literal(u32::from(QUANTIZER_DELTAS[4].unsigned_abs()), 4);
    encoder.write_literal(1, 1);
    // RFC 6386 section 19.2 retains token probabilities with refresh_entropy_probs 1.
    encoder.write_literal(1, 1);
    // RFC 6386 section 13.4 replaces a probability after each set update flag.
    for ((probability, default), selected) in COEFF_UPDATE_PROBS
        .into_iter()
        .zip(DEFAULT_COEFF_PROBS)
        .zip(probabilities.iter().copied())
    {
        let update = selected != default;
        encoder.write_bool(probability, update);
        if update {
            encoder.write_literal(u32::from(selected), 8);
        }
    }
    // RFC 6386 section 9.11 requires a probability when mb_no_skip_coeff is 1.
    encoder.write_bool(128, skip_probability.is_some());
    if let Some(probability) = skip_probability {
        encoder.write_literal(u32::from(probability), 8);
    }
}

struct ModeWriter {
    above: Vec<u8>,
    left: [u8; 4],
}

impl ModeWriter {
    fn new(macroblock_columns: usize) -> Self {
        Self {
            above: vec![0; macroblock_columns * 4],
            left: [0; 4],
        }
    }

    fn write_macroblock<W: EntropyWriter>(
        &mut self,
        encoder: &mut W,
        macroblock_x: usize,
        luma_mode: u8,
        chroma_mode: u8,
        subblock_modes: &[u8; 16],
    ) {
        // RFC 6386 section 11.3 gives missing left neighbors the DC mode.
        if macroblock_x == 0 {
            self.left.fill(0);
        }
        encoder.write_tree(&KF_Y_MODE_TREE, &KF_Y_MODE_PROBS, luma_mode, 0);
        if luma_mode == SUBBLOCK_MODE {
            self.write_subblock_modes(encoder, macroblock_x, subblock_modes);
        } else {
            let derived = derived_subblock_mode(luma_mode);
            self.above[macroblock_x * 4..macroblock_x * 4 + 4].fill(derived);
            self.left.fill(derived);
        }
        encoder.write_tree(&UV_MODE_TREE, &KF_UV_MODE_PROBS, chroma_mode, 0);
    }

    fn probabilities(&self, column: usize, row: usize) -> &'static [u8; 9] {
        &KF_B_MODE_PROBS[usize::from(self.above[column])][usize::from(self.left[row])]
    }

    fn write_subblock_modes<W: EntropyWriter>(
        &mut self,
        encoder: &mut W,
        macroblock_x: usize,
        modes: &[u8; 16],
    ) {
        for (block, &mode) in modes.iter().enumerate() {
            let column = macroblock_x * 4 + block % 4;
            let row = block / 4;
            encoder.write_tree(&B_MODE_TREE, self.probabilities(column, row), mode, 0);
            // RFC 6386 section 11.3 uses the preceding raster neighbors.
            self.above[column] = mode;
            self.left[row] = mode;
        }
    }
}

fn derived_subblock_mode(luma_mode: u8) -> u8 {
    // RFC 6386 section 11.3 maps each whole-block mode to a sub-block context.
    [0, 2, 3, 1][usize::from(luma_mode)]
}

struct ModeDecision {
    early_exit: bool,
    whole_modes: usize,
    subblocks: bool,
    #[cfg(test)]
    partition_limit: usize,
}

impl Default for ModeDecision {
    fn default() -> Self {
        Self {
            early_exit: true,
            whole_modes: 4,
            subblocks: true,
            #[cfg(test)]
            partition_limit: 524287,
        }
    }
}

impl ModeDecision {
    #[cfg(test)]
    fn set_early_exit(&mut self, enabled: bool) {
        self.early_exit = enabled;
    }

    fn finished(&self, distortion: u64) -> bool {
        // Each squared sample difference is at least zero, so their sum is at least zero.
        self.early_exit && distortion == 0
    }

    fn analyze(
        &self,
        source: &YuvPlanes,
        reconstruction: &mut YuvPlanes,
        macroblock: (usize, usize),
        quantization: QuantizationFactors,
    ) -> TokenMacroblock {
        let mut selected = TokenMacroblock::default();
        let mut best_distortion = u64::MAX;
        let mut best_luma = [0; 256];
        for (mode, &candidate) in CANDIDATES[..self.whole_modes].iter().enumerate() {
            let prediction = predict_luma(
                &reconstruction.y,
                source.y_stride,
                macroblock.0,
                macroblock.1,
                candidate,
            );
            let mut residual = MacroblockResidual::default();
            analyze_luma(
                &source.y,
                &prediction,
                &mut reconstruction.y,
                source.y_stride,
                macroblock,
                quantization,
                &mut residual,
            );
            let distortion =
                block_distortion::<16>(&source.y, &reconstruction.y, source.y_stride, macroblock);
            if distortion < best_distortion {
                best_distortion = distortion;
                selected.luma_mode = mode as u8;
                selected.residual = residual;
                copy_macroblock::<16>(
                    &reconstruction.y,
                    source.y_stride,
                    macroblock,
                    &mut best_luma,
                );
            }
            if self.finished(best_distortion) {
                break;
            }
        }
        if self.subblocks && !self.finished(best_distortion) {
            let mut candidate = TokenMacroblock {
                luma_mode: SUBBLOCK_MODE,
                ..TokenMacroblock::default()
            };
            let distortion = self.analyze_subblocks(
                source,
                reconstruction,
                macroblock,
                quantization,
                &mut candidate,
            );
            if distortion < best_distortion {
                selected = candidate;
                copy_macroblock::<16>(
                    &reconstruction.y,
                    source.y_stride,
                    macroblock,
                    &mut best_luma,
                );
            }
        }
        restore_macroblock::<16>(
            &mut reconstruction.y,
            source.y_stride,
            macroblock,
            &best_luma,
        );

        let mut best_distortion = u64::MAX;
        let mut best_u = [0; 64];
        let mut best_v = [0; 64];
        for (mode, &candidate) in CANDIDATES[..self.whole_modes].iter().enumerate() {
            let mut residual = MacroblockResidual::default();
            for (input, output, levels) in [
                (&source.u, &mut reconstruction.u, &mut residual.u),
                (&source.v, &mut reconstruction.v, &mut residual.v),
            ] {
                let prediction = predict_chroma(
                    output,
                    source.chroma_stride,
                    macroblock.0,
                    macroblock.1,
                    candidate,
                );
                analyze_chroma(
                    input,
                    &prediction,
                    output,
                    source.chroma_stride,
                    macroblock,
                    quantization,
                    levels,
                );
            }
            let distortion = block_distortion::<8>(
                &source.u,
                &reconstruction.u,
                source.chroma_stride,
                macroblock,
            ) + block_distortion::<8>(
                &source.v,
                &reconstruction.v,
                source.chroma_stride,
                macroblock,
            );
            if distortion < best_distortion {
                best_distortion = distortion;
                selected.chroma_mode = mode as u8;
                selected.residual.u = residual.u;
                selected.residual.v = residual.v;
                copy_macroblock::<8>(
                    &reconstruction.u,
                    source.chroma_stride,
                    macroblock,
                    &mut best_u,
                );
                copy_macroblock::<8>(
                    &reconstruction.v,
                    source.chroma_stride,
                    macroblock,
                    &mut best_v,
                );
            }
            if self.finished(best_distortion) {
                break;
            }
        }
        restore_macroblock::<8>(
            &mut reconstruction.u,
            source.chroma_stride,
            macroblock,
            &best_u,
        );
        restore_macroblock::<8>(
            &mut reconstruction.v,
            source.chroma_stride,
            macroblock,
            &best_v,
        );
        selected
    }

    fn analyze_subblocks(
        &self,
        source: &YuvPlanes,
        reconstruction: &mut YuvPlanes,
        macroblock: (usize, usize),
        quantization: QuantizationFactors,
        selected: &mut TokenMacroblock,
    ) -> u64 {
        let mut total = 0;
        for block in 0..16 {
            let x = macroblock.0 * 16 + block % 4 * 4;
            let y = macroblock.1 * 16 + block / 4 * 4;
            let mut best_distortion = u64::MAX;
            let mut best_pixels = [0; 16];
            for (mode, candidate) in CANDIDATES[4..].iter().enumerate() {
                let mut prediction = [0; 16];
                candidate.predict::<4>(&reconstruction.y, source.y_stride, x, y, &mut prediction);
                let samples =
                    residual_block(&source.y, source.y_stride, (x, y), &prediction, 4, (0, 0));
                let levels =
                    quantize_block(&forward_dct(&samples), quantization.y_dc, quantization.y_ac);
                let coefficients = dequantize_block(&levels, quantization.y_dc, quantization.y_ac);
                let residual = inverse_dct(&coefficients);
                let pixels = core::array::from_fn::<_, 16, _>(|position| {
                    clamped_add(prediction[position], residual[position])
                });
                let distortion = pixels
                    .iter()
                    .enumerate()
                    .map(|(position, &pixel)| {
                        let difference = i32::from(
                            source.y[(y + position / 4) * source.y_stride + x + position % 4],
                        ) - i32::from(pixel);
                        (difference * difference) as u64
                    })
                    .sum();
                if distortion < best_distortion {
                    best_distortion = distortion;
                    best_pixels = pixels;
                    selected.subblock_modes[block] = mode as u8;
                    selected.residual.y[block] = levels;
                }
                if self.finished(best_distortion) {
                    break;
                }
            }
            // RFC 6386 section 12.3 predicts each sub-block from reconstructed neighbors.
            restore_macroblock::<4>(
                &mut reconstruction.y,
                source.y_stride,
                (x / 4, y / 4),
                &best_pixels,
            );
            total += best_distortion;
        }
        total
    }
}

fn block_distortion<const SIDE: usize>(
    source: &[u8],
    reconstruction: &[u8],
    stride: usize,
    macroblock: (usize, usize),
) -> u64 {
    let mut distortion = 0;
    for row in 0..SIDE {
        let offset = (macroblock.1 * SIDE + row) * stride + macroblock.0 * SIDE;
        for column in 0..SIDE {
            let difference =
                i32::from(source[offset + column]) - i32::from(reconstruction[offset + column]);
            distortion += (difference * difference) as u64;
        }
    }
    distortion
}

fn copy_macroblock<const SIDE: usize>(
    plane: &[u8],
    stride: usize,
    macroblock: (usize, usize),
    output: &mut [u8],
) {
    for row in 0..SIDE {
        let offset = (macroblock.1 * SIDE + row) * stride + macroblock.0 * SIDE;
        output[row * SIDE..(row + 1) * SIDE].copy_from_slice(&plane[offset..offset + SIDE]);
    }
}

fn restore_macroblock<const SIDE: usize>(
    plane: &mut [u8],
    stride: usize,
    macroblock: (usize, usize),
    pixels: &[u8],
) {
    for row in 0..SIDE {
        let offset = (macroblock.1 * SIDE + row) * stride + macroblock.0 * SIDE;
        plane[offset..offset + SIDE].copy_from_slice(&pixels[row * SIDE..(row + 1) * SIDE]);
    }
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

fn start_riff(
    pixels: &[u8],
    width: usize,
    height: usize,
    bytes_per_pixel: usize,
    options: &Options,
    partition_capacity: usize,
) -> (Vec<u8>, usize) {
    let has_alpha = bytes_per_pixel == 4
        && options.alpha == Alpha::Lossless
        && pixels[3..].iter().step_by(4).any(|value| *value != 255);
    let extended = has_alpha || options.force_vp8x;
    let alpha_size = usize::from(has_alpha) * (1 + width * height);
    let alpha_padding = alpha_size & 1;
    let extended_size = usize::from(extended) * 18;
    let alpha_chunk_size = usize::from(has_alpha) * (8 + alpha_size + alpha_padding);
    let mut output =
        Vec::with_capacity(30 + partition_capacity + 1 + extended_size + alpha_chunk_size);
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

    let chunk_start = output.len();
    output.extend_from_slice(b"VP8 ");
    output.extend_from_slice(&[0; 7]);
    output.extend_from_slice(&[0x9d, 0x01, 0x2a]);
    output.extend_from_slice(&(width as u16).to_le_bytes());
    output.extend_from_slice(&(height as u16).to_le_bytes());
    (output, chunk_start)
}

#[cfg(test)]
mod tests {
    use super::{
        encode, write_frame_header, KF_UV_MODE_PROBS, KF_Y_MODE_PROBS, KF_Y_MODE_TREE, UV_MODE_TREE,
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
    fn the_subblock_probability_table_matches_every_dimension_and_checksum() {
        assert_eq!(super::KF_B_MODE_PROBS.len(), 10);
        for above in super::KF_B_MODE_PROBS {
            assert_eq!(above.len(), 10);
            for left in above {
                assert_eq!(left.len(), 9);
            }
        }
        let entries: Vec<u8> = super::KF_B_MODE_PROBS
            .into_iter()
            .flatten()
            .flatten()
            .collect();
        assert_eq!(entries.len(), 900);
        assert_eq!(entries.first(), Some(&231));
        assert_eq!(entries.last(), Some(&24));
        assert_eq!(
            entries.iter().map(|&value| u32::from(value)).sum::<u32>(),
            77557
        );
    }

    #[test]
    fn the_subblock_tree_codes_each_mode_with_its_section_11_2_path() {
        assert_eq!(super::B_MODE_TREE.len(), 18);
        let paths = [
            "0", "10", "110", "11100", "11110", "111010", "111011", "111110", "1111110", "1111111",
        ];
        for (mode, expected) in paths.into_iter().enumerate() {
            let trace = path(&super::B_MODE_TREE, &[128; 9], mode as u8);
            let expected: Vec<(u8, bool)> =
                expected.bytes().map(|bit| (128, bit == b'1')).collect();
            assert_eq!(trace, expected, "mode {mode}");
        }
    }

    #[test]
    fn whole_block_modes_set_the_derived_bottom_and_right_contexts() {
        let mut writer = super::ModeWriter::new(2);
        let mut encoder = BoolEncoder::new();
        for (luma, expected) in [0, 2, 3, 1].into_iter().enumerate() {
            assert_eq!(super::derived_subblock_mode(luma as u8), expected);
            writer.write_macroblock(&mut encoder, 1, luma as u8, 0, &[9; 16]);
            assert_eq!(writer.above[..4], [0; 4]);
            assert_eq!(writer.above[4..], [expected; 4]);
            assert_eq!(writer.left, [expected; 4]);
        }
    }

    #[test]
    fn the_context_row_and_column_select_above_then_left_probabilities() {
        let writer = super::ModeWriter {
            above: vec![1, 2, 3, 4],
            left: [5, 6, 7, 8],
        };
        assert_eq!(
            *writer.probabilities(0, 0),
            [74, 43, 26, 146, 73, 166, 49, 23, 157]
        );
        assert_eq!(
            *writer.probabilities(1, 1),
            [34, 22, 116, 206, 23, 34, 43, 166, 73]
        );
        assert_eq!(
            *writer.probabilities(2, 2),
            [142, 78, 78, 16, 255, 128, 34, 197, 171]
        );
        assert_eq!(
            *writer.probabilities(3, 3),
            [57, 18, 10, 102, 102, 213, 34, 20, 43]
        );
    }

    #[test]
    fn subblock_records_follow_raster_neighbors_and_retain_boundary_modes() {
        let mut writer = super::ModeWriter {
            above: vec![9, 9, 9, 9, 1, 2, 3, 4],
            left: [5, 6, 7, 8],
        };
        let modes = [1, 2, 3, 4, 5, 6, 7, 8, 9, 0, 1, 2, 3, 4, 5, 6];
        let neighbors = [
            (1, 5),
            (2, 1),
            (3, 2),
            (4, 3),
            (1, 6),
            (2, 5),
            (3, 6),
            (4, 7),
            (5, 7),
            (6, 9),
            (7, 0),
            (8, 1),
            (9, 8),
            (0, 3),
            (1, 4),
            (2, 5),
        ];
        let mut encoder = BoolEncoder::new();
        writer.write_macroblock(&mut encoder, 1, 4, 2, &modes);
        let bytes = encoder.finish();
        let mut decoder = crate::bool_coder::BoolDecoder::new(&bytes);
        assert_eq!(decoder.read_tree(&KF_Y_MODE_TREE, &KF_Y_MODE_PROBS, 0), 4);
        for ((above, left), mode) in neighbors.into_iter().zip(modes) {
            assert_eq!(
                decoder.read_tree(&super::B_MODE_TREE, &super::KF_B_MODE_PROBS[above][left], 0),
                mode
            );
        }
        assert_eq!(decoder.read_tree(&UV_MODE_TREE, &KF_UV_MODE_PROBS, 0), 2);
        assert_eq!(writer.above, [9, 9, 9, 9, 3, 4, 5, 6]);
        assert_eq!(writer.left, [4, 8, 2, 6]);
    }

    #[test]
    fn frame_and_row_boundaries_use_dc_for_missing_neighbors() {
        let mut writer = super::ModeWriter::new(2);
        assert_eq!(writer.above, [0; 8]);
        assert_eq!(writer.left, [0; 4]);
        let mut encoder = BoolEncoder::new();
        writer.write_macroblock(&mut encoder, 0, 1, 0, &[0; 16]);
        writer.write_macroblock(&mut encoder, 1, 2, 0, &[0; 16]);
        assert_eq!(writer.above, [2, 2, 2, 2, 3, 3, 3, 3]);
        assert_eq!(writer.left, [3; 4]);
        let mut encoder = BoolEncoder::new();
        writer.write_macroblock(&mut encoder, 0, 4, 0, &[1; 16]);
        let bytes = encoder.finish();
        let mut decoder = crate::bool_coder::BoolDecoder::new(&bytes);
        assert_eq!(decoder.read_tree(&KF_Y_MODE_TREE, &KF_Y_MODE_PROBS, 0), 4);
        for block in 0..16 {
            let above = if block < 4 { 2 } else { 1 };
            let left = if block % 4 == 0 { 0 } else { 1 };
            assert_eq!(
                decoder.read_tree(&super::B_MODE_TREE, &super::KF_B_MODE_PROBS[above][left], 0),
                1
            );
        }
        assert_eq!(decoder.read_tree(&UV_MODE_TREE, &KF_UV_MODE_PROBS, 0), 0);
        assert_eq!(writer.above, [1, 1, 1, 1, 3, 3, 3, 3]);
        assert_eq!(writer.left, [1; 4]);
    }

    #[test]
    fn empty_counts_keep_every_default_probability() {
        assert_eq!(
            super::TokenStatistics::default().probabilities(),
            crate::residual::DEFAULT_COEFF_PROBS
        );
    }

    #[test]
    fn an_opaque_file_carries_one_vp8_chunk_with_the_frame_fields() {
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
        assert_eq!(encoded.get(20 + chunk_size), None);
        assert_eq!(frame_tag & 1, 0);
        assert_eq!((frame_tag >> 1) & 7, 0);
        assert_eq!((frame_tag >> 4) & 1, 1);
        assert_eq!(first_partition_size, 14);
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
        assert_eq!(read_deltas(&mut header), [0, 0, 0, -2, -4]);
        assert_eq!(header.read_literal(1), 1);
        let probabilities = read_updates(&mut header);
        let updates: Vec<_> = probabilities
            .into_iter()
            .zip(crate::residual::DEFAULT_COEFF_PROBS)
            .enumerate()
            .filter_map(|(index, (sent, default))| (sent != default).then_some((index, sent)))
            .collect();
        assert_eq!(updates, [(528, 1)]);
        assert_eq!(header.read_literal(1), 0);
        assert_eq!(
            read_frame_modes(&encoded, 1, 1),
            [(4, 2, [1, 3, 1, 1, 2, 1, 0, 0, 2, 0, 0, 0, 1, 0, 0, 0])],
        );
    }

    #[test]
    fn filter_values_saturate_at_the_header_field_limits() {
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
    }

    #[test]
    fn the_auto_filter_signals_the_quantizer_index_capped_at_sixty_three() {
        for index in 0..=127 {
            let encoded = encode(&[128; 3], 1, 1, 3, index, &Options::default());
            assert_eq!(read_filter_fields(&encoded.webp), (index.min(63), 0));
        }
    }

    #[test]
    fn the_off_filter_signals_level_zero_and_sharpness_zero() {
        let options = Options {
            filter: Filter::Off,
            ..Options::default()
        };
        for index in 0..=127 {
            let encoded = encode(&[128; 3], 1, 1, 3, index, &options);
            assert_eq!(read_filter_fields(&encoded.webp), (0, 0));
        }
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
        write_frame_header(
            &mut partition,
            0,
            Filter::Off,
            &crate::residual::DEFAULT_COEFF_PROBS,
            None,
        );
        for _ in 0..macroblock_count {
            partition.write_tree(&KF_Y_MODE_TREE, &KF_Y_MODE_PROBS, 0, 0);
            partition.write_tree(&UV_MODE_TREE, &KF_UV_MODE_PROBS, 0, 0);
        }
        let partition_size = partition.finish().len();
        assert_eq!(partition_size, 449_398);
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
                decoder
                    .read_image(&mut pixels)
                    .expect("decode the fixture pixels");
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
        let directory =
            scratch_directory("dwebp_yuv_output_ends_with_each_input_alpha_plane_at_each_quality");
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
                assert!(status.success(), "{} q{quality}", fixture.name);
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
        let directory =
            scratch_directory("dwebp_decodes_every_fixture_at_its_dimensions_at_each_quality");
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
                assert!(status.success(), "{} q{quality}", fixture.name);
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
        let directory =
            scratch_directory("reconstruction_matches_dwebp_for_every_fixture_quality_and_index");
        for fixture in generator::all() {
            for quality in [0u8, 25, 50, 75, 90, 95, 100] {
                assert_reconstruction(&directory, &fixture, quantizer_index(quality));
            }
        }
        for name in ["gradient", "noise", "diagonals"] {
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
    fn auto_filtered_reconstruction_matches_dwebp_for_every_fixture_quality_and_index() {
        if !oracle_is_available("dwebp") {
            return;
        }
        let directory = scratch_directory(
            "auto_filtered_reconstruction_matches_dwebp_for_every_fixture_quality_and_index",
        );
        let fixtures = generator::all();
        for fixture in &fixtures {
            for quality in [0u8, 25, 50, 75, 90, 95, 100] {
                assert_reconstruction_with_filter(
                    &directory,
                    fixture,
                    quantizer_index(quality),
                    Filter::Auto,
                );
            }
        }
        for name in ["gradient", "noise", "diagonals"] {
            let fixture = fixtures
                .iter()
                .find(|fixture| fixture.name == name)
                .expect("find the indexed fixture");
            for index in 0..=127 {
                assert_reconstruction_with_filter(&directory, fixture, index, Filter::Auto);
            }
        }
        fs::remove_dir_all(directory).expect("remove the test directory");
    }

    #[test]
    fn explicit_filter_levels_and_sharpness_match_dwebp_on_lowpass_noise() {
        if !oracle_is_available("dwebp") {
            return;
        }
        let directory =
            scratch_directory("explicit_filter_levels_and_sharpness_match_dwebp_on_lowpass_noise");
        let fixtures = generator::all();
        let fixture = fixtures
            .iter()
            .find(|fixture| fixture.name == "lowpass-noise")
            .expect("find the lowpass fixture");
        for (level, sharpness) in (0..=63)
            .map(|level| (level, 0))
            .chain((0..=7).map(|sharpness| (30, sharpness)))
        {
            let options = Options {
                quality: 50,
                filter: Filter::Level { level, sharpness },
                ..Options::default()
            };
            let encoded = encode(
                &fixture.rgba,
                fixture.width as usize,
                fixture.height as usize,
                4,
                quantizer_index(50),
                &options,
            );
            assert_eq!(read_filter_fields(&encoded.webp), (level, sharpness));
            assert_eq!(
                encoded.webp,
                crate::encode_rgba(&fixture.rgba, fixture.width, fixture.height, &options)
                    .expect("encode the filtered fixture")
            );
            assert_encoded_reconstruction(&directory, fixture, quantizer_index(50), &encoded);
        }
        fs::remove_dir_all(directory).expect("remove the test directory");
    }

    #[test]
    fn every_swept_dimension_decodes_and_matches_reconstruction_and_boundary_sides_return_errors() {
        let oracle_available = oracle_is_available("dwebp");
        let directory = scratch_directory(
            "every_swept_dimension_decodes_and_matches_reconstruction_and_boundary_sides_return_errors",
        );
        for quality in [0u8, 50, 100] {
            for width in 1..=48u32 {
                for height in 1..=48u32 {
                    let fixture = generator::noise("dimension-sweep", width, height);
                    assert_dimension_case(&directory, &fixture, quality, oracle_available);
                }
            }
        }
        for (width, height) in [
            (16383, 1),
            (1, 16383),
            (16383, 3),
            (3, 16383),
            (4097, 1),
            (1, 4097),
        ] {
            let fixture = generator::noise("long-dimension-sweep", width, height);
            assert_dimension_case(&directory, &fixture, 50, oracle_available);
        }

        let options = Options::default();
        for (width, height) in [
            (0u32, 8u32),
            (8, 0),
            (0, 0),
            (16384, 8),
            (8, 16384),
            (16384, 16384),
        ] {
            assert_eq!(
                crate::encode_rgba(&[], width, height, &options),
                Err(crate::Error::DimensionsOutOfRange { width, height })
            );
            assert_eq!(
                crate::encode_rgb(&[], width, height, &options),
                Err(crate::Error::DimensionsOutOfRange { width, height })
            );
        }
        fs::remove_dir_all(directory).expect("remove the test directory");
    }

    #[test]
    fn cwebp_writes_each_checked_in_quality_index_into_its_frame_header() {
        if !oracle_is_available("cwebp") {
            return;
        }
        let directory =
            scratch_directory("cwebp_writes_each_checked_in_quality_index_into_its_frame_header");
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
            assert!(status.success(), "quality {quality}");
            let webp = fs::read(&output).expect("read the cwebp output");
            assert_eq!(read_quantizer_index(&webp), *expected);
        }
        fs::remove_dir_all(directory).expect("remove the test directory");
    }

    #[test]
    fn mode_decision_minimizes_reconstruction_error_for_luma_and_joint_chroma() {
        let source = crate::color::YuvPlanes {
            y: (0..1024)
                .map(|i| (16 + (37 * i + i / 32 * 11) % 220) as u8)
                .collect(),
            u: (0..256)
                .map(|i| (16 + (23 * i + i / 16 * 7) % 225) as u8)
                .collect(),
            v: (0..256)
                .map(|i| (16 + (41 * i + i / 16 * 13) % 225) as u8)
                .collect(),
            y_stride: 32,
            chroma_stride: 16,
        };
        let mut reconstruction = crate::color::YuvPlanes {
            y: (0..1024).map(|i| (i * 19 % 256) as u8).collect(),
            u: (0..256).map(|i| (i * 31 % 256) as u8).collect(),
            v: (0..256).map(|i| (i * 53 % 256) as u8).collect(),
            y_stride: 32,
            chroma_stride: 16,
        };
        let quantization = factors(57);
        let mut luma_scores = [0; 5];
        let mut u_scores = [0; 4];
        let mut v_scores = [0; 4];
        for (mode, &candidate) in crate::prediction::CANDIDATES[..4].iter().enumerate() {
            let prediction =
                crate::prediction::predict_luma(&reconstruction.y, 32, 1, 1, candidate);
            let mut residual = crate::residual::MacroblockResidual::default();
            super::analyze_luma(
                &source.y,
                &prediction,
                &mut reconstruction.y,
                32,
                (1, 1),
                quantization,
                &mut residual,
            );
            luma_scores[mode] =
                super::block_distortion::<16>(&source.y, &reconstruction.y, 32, (1, 1));
            for (input, output, scores) in [
                (&source.u, &mut reconstruction.u, &mut u_scores),
                (&source.v, &mut reconstruction.v, &mut v_scores),
            ] {
                let prediction = crate::prediction::predict_chroma(output, 16, 1, 1, candidate);
                super::analyze_chroma(
                    input,
                    &prediction,
                    output,
                    16,
                    (1, 1),
                    quantization,
                    &mut [[0; 16]; 4],
                );
                scores[mode] = super::block_distortion::<8>(input, output, 16, (1, 1));
            }
        }
        let decision = super::ModeDecision::default();
        luma_scores[4] = decision.analyze_subblocks(
            &source,
            &mut reconstruction,
            (1, 1),
            quantization,
            &mut crate::residual::TokenMacroblock::default(),
        );
        let selected = decision.analyze(&source, &mut reconstruction, (1, 1), quantization);
        assert_eq!(luma_scores, [16861, 16709, 17237, 19288, 13603]);
        assert_eq!(u_scores, [2456, 2744, 2620, 3127]);
        assert_eq!(v_scores, [1667, 1155, 1748, 4242]);
        assert_eq!((selected.luma_mode, selected.chroma_mode), (4, 1));
        assert_eq!(
            super::block_distortion::<16>(&source.y, &reconstruction.y, 32, (1, 1)),
            *luma_scores.iter().min().unwrap()
        );
        let chroma_scores = core::array::from_fn::<_, 4, _>(|mode| u_scores[mode] + v_scores[mode]);
        assert_eq!(
            super::block_distortion::<8>(&source.u, &reconstruction.u, 16, (1, 1))
                + super::block_distortion::<8>(&source.v, &reconstruction.v, 16, (1, 1)),
            *chroma_scores.iter().min().unwrap(),
        );
    }

    #[test]
    fn zero_distortion_ties_keep_dc_with_either_early_exit_setting() {
        for early_exit in [true, false] {
            let source = crate::color::YuvPlanes {
                y: vec![128; 256],
                u: vec![128; 64],
                v: vec![128; 64],
                y_stride: 16,
                chroma_stride: 8,
            };
            let mut reconstruction = crate::color::YuvPlanes {
                y: vec![0; 256],
                u: vec![0; 64],
                v: vec![0; 64],
                y_stride: 16,
                chroma_stride: 8,
            };
            let mut decision = super::ModeDecision::default();
            decision.set_early_exit(early_exit);
            let selected = decision.analyze(&source, &mut reconstruction, (0, 0), factors(0));
            assert_eq!(selected, crate::residual::TokenMacroblock::default());
            assert_eq!(reconstruction.y, [128; 256]);
            assert_eq!(reconstruction.u, [128; 64]);
            assert_eq!(reconstruction.v, [128; 64]);
        }
    }

    #[test]
    fn early_exits_preserve_fixture_digests_at_every_fixed_quality_and_entry_point() {
        use sha2::{Digest, Sha256};
        for fixture in generator::all() {
            let rgb = rgb_bytes(&fixture.rgba);
            for quality in [0u8, 25, 50, 75, 90, 95, 100] {
                let options = Options {
                    quality,
                    filter: Filter::Off,
                    ..Options::default()
                };
                for (pixels, bytes_per_pixel) in [(&fixture.rgba, 4), (&rgb, 3)] {
                    let mut exhaustive = super::ModeDecision::default();
                    exhaustive.set_early_exit(false);
                    let expected = super::encode_with_decision(
                        pixels,
                        fixture.width as usize,
                        fixture.height as usize,
                        bytes_per_pixel,
                        quantizer_index(quality),
                        &options,
                        exhaustive,
                    );
                    let actual = encode(
                        pixels,
                        fixture.width as usize,
                        fixture.height as usize,
                        bytes_per_pixel,
                        quantizer_index(quality),
                        &options,
                    );
                    assert_eq!(
                        Sha256::digest(&actual.webp),
                        Sha256::digest(&expected.webp),
                        "{} q{quality} channels {bytes_per_pixel}",
                        fixture.name,
                    );
                    assert_eq!(actual.reconstruction.y, expected.reconstruction.y);
                    assert_eq!(actual.reconstruction.u, expected.reconstruction.u);
                    assert_eq!(actual.reconstruction.v, expected.reconstruction.v);
                }
            }
        }
    }

    #[test]
    fn diagonals_codes_four_distinct_subblock_modes_in_one_macroblock() {
        let fixtures = generator::all();
        let fixture = fixtures
            .iter()
            .find(|fixture| fixture.name == "diagonals")
            .unwrap();
        let encoded = encode(
            &fixture.rgba,
            fixture.width as usize,
            fixture.height as usize,
            4,
            quantizer_index(75),
            &Options::default(),
        );
        let modes = read_frame_modes(&encoded.webp, 3, 3);
        let distinct: Vec<_> = modes
            .iter()
            .filter(|(mode, _, _)| *mode == 4)
            .map(|(_, _, blocks)| {
                blocks
                    .iter()
                    .fold(0u16, |mask, &mode| mask | 1 << mode)
                    .count_ones()
            })
            .collect();
        assert_eq!(distinct, [5, 3, 4, 4, 4]);
        assert_eq!(
            modes[0],
            (4, 0, [1, 2, 0, 0, 3, 3, 2, 0, 1, 3, 3, 6, 1, 0, 3, 3])
        );
    }

    fn read_frame_modes(webp: &[u8], columns: usize, rows: usize) -> Vec<(u8, u8, [u8; 16])> {
        read_frame_records(webp, columns, rows).0
    }

    type FrameRecords = (Vec<(u8, u8, [u8; 16])>, Vec<bool>, [u8; 1056]);

    fn read_frame_records(webp: &[u8], columns: usize, rows: usize) -> FrameRecords {
        let payload = vp8_payload(webp);
        let mut decoder = crate::bool_coder::BoolDecoder::new(&payload[10..]);
        for _ in 0..4 {
            assert_eq!(decoder.read_literal(1), 0);
        }
        decoder.read_literal(6);
        decoder.read_literal(3);
        assert_eq!(decoder.read_literal(1), 0);
        assert_eq!(decoder.read_literal(2), 0);
        decoder.read_literal(7);
        assert_eq!(read_deltas(&mut decoder), [0, 0, 0, -2, -4]);
        assert_eq!(decoder.read_literal(1), 1);
        let probabilities = read_updates(&mut decoder);
        let mut skips = Vec::new();
        let skip = if decoder.read_bool(128) {
            Some(decoder.read_literal(8) as u8)
        } else {
            None
        };
        let mut blocks = vec![0usize; columns * rows * 16];
        let mut result = Vec::new();
        for row in 0..rows {
            for column in 0..columns {
                skips.push(skip.is_some_and(|probability| decoder.read_bool(probability)));
                let luma = decoder.read_tree(&KF_Y_MODE_TREE, &KF_Y_MODE_PROBS, 0);
                let mut modes = [0; 16];
                for (block, mode) in modes.iter_mut().enumerate() {
                    let x = column * 4 + block % 4;
                    let y = row * 4 + block / 4;
                    let position = y * columns * 4 + x;
                    if luma == 4 {
                        let above = if y == 0 {
                            0
                        } else {
                            blocks[position - columns * 4]
                        };
                        let left = if x == 0 { 0 } else { blocks[position - 1] };
                        *mode = decoder.read_tree(
                            &super::B_MODE_TREE,
                            &super::KF_B_MODE_PROBS[above][left],
                            0,
                        );
                    } else {
                        *mode = [0, 2, 3, 1][usize::from(luma)];
                    }
                    blocks[position] = usize::from(*mode);
                }
                let chroma = decoder.read_tree(&UV_MODE_TREE, &KF_UV_MODE_PROBS, 0);
                result.push((luma, chroma, modes));
            }
        }
        (result, skips, probabilities)
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

    fn assert_dimension_case(
        directory: &Path,
        fixture: &generator::Fixture,
        quality: u8,
        oracle_available: bool,
    ) {
        let options = Options {
            quality,
            filter: Filter::Off,
            ..Options::default()
        };
        let rgb = rgb_bytes(&fixture.rgba);
        let rgba_webp = crate::encode_rgba(&fixture.rgba, fixture.width, fixture.height, &options)
            .expect("encode the RGBA dimension case");
        let rgb_webp = crate::encode_rgb(&rgb, fixture.width, fixture.height, &options)
            .expect("encode the RGB dimension case");
        assert_image_webp_decodes(&rgba_webp, fixture, quality, "RGBA");
        assert_image_webp_decodes(&rgb_webp, fixture, quality, "RGB");

        if oracle_available {
            assert_reconstruction_for_pixels(
                directory,
                fixture,
                quality,
                &fixture.rgba,
                4,
                &rgba_webp,
                "RGBA",
            );
            assert_reconstruction_for_pixels(
                directory, fixture, quality, &rgb, 3, &rgb_webp, "RGB",
            );
        }
    }

    fn assert_image_webp_decodes(
        webp: &[u8],
        fixture: &generator::Fixture,
        quality: u8,
        entry: &str,
    ) {
        let mut decoder = image_webp::WebPDecoder::new(Cursor::new(webp))
            .expect("decode the dimension case header");
        assert_eq!(
            decoder.dimensions(),
            (fixture.width, fixture.height),
            "{entry} {}x{} q{quality}",
            fixture.width,
            fixture.height
        );
        assert_eq!(
            u8::from(decoder.has_alpha()),
            0,
            "{entry} {}x{} q{quality}",
            fixture.width,
            fixture.height
        );
        let mut pixels = vec![0; fixture.width as usize * fixture.height as usize * 3];
        decoder
            .read_image(&mut pixels)
            .expect("decode the dimension case pixels");
    }

    fn assert_reconstruction_for_pixels(
        directory: &Path,
        fixture: &generator::Fixture,
        quality: u8,
        pixels: &[u8],
        bytes_per_pixel: usize,
        expected_webp: &[u8],
        entry: &str,
    ) {
        let options = Options {
            filter: Filter::Off,
            ..Options::default()
        };
        let index = quantizer_index(quality);
        let encoded = encode(
            pixels,
            fixture.width as usize,
            fixture.height as usize,
            bytes_per_pixel,
            index,
            &options,
        );
        assert_eq!(
            encoded.webp, expected_webp,
            "{entry} {}x{} q{quality}",
            fixture.width, fixture.height
        );
        assert_encoded_reconstruction(directory, fixture, index, &encoded);
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
        crate::png_writer::write(
            output,
            fixture.width,
            fixture.height,
            png::ColorType::Rgba,
            &fixture.rgba,
        )
        .expect("write the PNG pixels");
    }

    fn assert_reconstruction(directory: &Path, fixture: &generator::Fixture, index: u8) {
        assert_reconstruction_with_filter(directory, fixture, index, Filter::Off);
    }

    fn assert_reconstruction_with_filter(
        directory: &Path,
        fixture: &generator::Fixture,
        index: u8,
        filter: Filter,
    ) {
        let options = Options {
            filter,
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
        let expected = match filter {
            Filter::Auto => (index.min(63), 0),
            Filter::Off => (0, 0),
            Filter::Level { level, sharpness } => (level.min(63), sharpness.min(7)),
        };
        assert_eq!(read_filter_fields(&encoded.webp), expected);
        assert_encoded_reconstruction(directory, fixture, index, &encoded);
    }

    fn assert_encoded_reconstruction(
        directory: &Path,
        fixture: &generator::Fixture,
        index: u8,
        encoded: &super::EncodedFrame,
    ) {
        let mut decoder = image_webp::WebPDecoder::new(Cursor::new(&encoded.webp))
            .expect("decode the fixture header");
        assert_eq!(decoder.dimensions(), (fixture.width, fixture.height));
        let alpha = has_nonopaque_alpha(&fixture.rgba);
        assert_eq!(decoder.has_alpha(), alpha);
        let channels = if alpha { 4 } else { 3 };
        let mut pixels = vec![0; fixture.width as usize * fixture.height as usize * channels];
        decoder
            .read_image(&mut pixels)
            .expect("decode the fixture pixels");
        if alpha {
            assert_eq!(alpha_bytes(&pixels), alpha_bytes(&fixture.rgba));
        }
        let png = directory.join("dimensions.png");
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
        assert!(
            status.success(),
            "fixture {}, quantizer index {index}",
            fixture.name
        );
        let status = Command::new("dwebp")
            .arg("-quiet")
            .arg(&input)
            .arg("-o")
            .arg(&png)
            .status()
            .expect("decode the fixture to PNG");
        assert_eq!(status.code(), Some(0));
        assert_eq!(png_dimensions(&png), (fixture.width, fixture.height));
        let decoded = fs::read(output).expect("read the decoded YUV planes");
        let width = fixture.width as usize;
        let height = fixture.height as usize;
        let chroma_width = (fixture.width as usize).div_ceil(2);
        let chroma_height = (fixture.height as usize).div_ceil(2);
        let (level, sharpness) = read_filter_fields(&encoded.webp);
        let columns = width.div_ceil(16);
        let rows = height.div_ceil(16);
        let (modes, skips, _) = read_frame_records(&encoded.webp, columns, rows);
        let macroblocks: Vec<_> = modes
            .iter()
            .zip(skips)
            .map(|(mode, skip)| crate::loop_filter::Macroblock {
                skip,
                b_pred: mode.0 == 4,
            })
            .collect();
        let mut y = encoded.reconstruction.y.clone();
        let mut u = encoded.reconstruction.u.clone();
        let mut v = encoded.reconstruction.v.clone();
        crate::loop_filter::filter_frame(
            [&mut y, &mut u, &mut v],
            columns,
            rows,
            level,
            sharpness,
            &macroblocks,
        );
        let y_length = width * height;
        let chroma_length = chroma_width * chroma_height;
        assert_plane_matches(
            fixture,
            (index, level, sharpness),
            "Y",
            &y,
            encoded.reconstruction.y_stride,
            &decoded[..y_length],
            (width, height),
        );
        assert_plane_matches(
            fixture,
            (index, level, sharpness),
            "U",
            &u,
            encoded.reconstruction.chroma_stride,
            &decoded[y_length..y_length + chroma_length],
            (chroma_width, chroma_height),
        );
        assert_plane_matches(
            fixture,
            (index, level, sharpness),
            "V",
            &v,
            encoded.reconstruction.chroma_stride,
            &decoded[y_length + chroma_length..y_length + 2 * chroma_length],
            (chroma_width, chroma_height),
        );
        if has_nonopaque_alpha(&fixture.rgba) {
            let alpha = alpha_bytes(&fixture.rgba);
            assert_plane_matches(
                fixture,
                (index, level, sharpness),
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
        controls: (u8, u8, u8),
        plane: &str,
        reconstruction: &[u8],
        reconstruction_stride: usize,
        decoded: &[u8],
        dimensions: (usize, usize),
    ) {
        let (width, height) = dimensions;
        let (index, level, sharpness) = controls;
        for row in 0..height {
            for column in 0..width {
                let decoded_value = decoded[row * width + column];
                let reconstruction_value = reconstruction[row * reconstruction_stride + column];
                assert_eq!(
                    decoded_value, reconstruction_value,
                    "fixture {}, quantizer index {index}, level {level}, sharpness {sharpness}, plane {plane}, first differing row {row}, column {column}",
                    fixture.name
                );
            }
        }
    }

    fn read_filter_fields(webp: &[u8]) -> (u8, u8) {
        let payload = vp8_payload(webp);
        let mut decoder = crate::bool_coder::BoolDecoder::new(&payload[10..]);
        for _ in 0..4 {
            assert_eq!(decoder.read_literal(1), 0);
        }
        (decoder.read_literal(6) as u8, decoder.read_literal(3) as u8)
    }

    fn read_quantizer_index(webp: &[u8]) -> u8 {
        read_quantizer_fields(webp).0
    }

    fn read_quantizer_fields(webp: &[u8]) -> (u8, [i8; 5]) {
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
        let index = decoder.read_literal(7) as u8;
        (index, read_deltas(&mut decoder))
    }
    fn read_deltas(decoder: &mut crate::bool_coder::BoolDecoder<'_>) -> [i8; 5] {
        core::array::from_fn(|_| {
            if decoder.read_bool(128) {
                let magnitude = decoder.read_literal(4) as i8;
                if decoder.read_bool(128) {
                    -magnitude
                } else {
                    magnitude
                }
            } else {
                0
            }
        })
    }

    fn read_updates(decoder: &mut crate::bool_coder::BoolDecoder<'_>) -> [u8; 1056] {
        let mut probabilities = crate::residual::DEFAULT_COEFF_PROBS;
        for (selected, update) in probabilities
            .iter_mut()
            .zip(crate::residual::COEFF_UPDATE_PROBS)
        {
            if decoder.read_bool(update) {
                *selected = decoder.read_literal(8) as u8;
            }
        }
        probabilities
    }

    #[test]
    fn cwebp_writes_the_chroma_deltas_and_quality_index_for_every_fixture() {
        if !oracle_is_available("cwebp") {
            return;
        }
        let directory =
            scratch_directory("cwebp_writes_the_chroma_deltas_and_quality_index_for_every_fixture");
        let input = directory.join("input.png");
        let output = directory.join("output.webp");
        for fixture in generator::all() {
            write_png(&input, &fixture);
            for quality in [25, 50, 75, 90, 95] {
                let status = Command::new("cwebp")
                    .args(["-quiet", "-q", &quality.to_string(), "-segments", "1"])
                    .arg(&input)
                    .arg("-o")
                    .arg(&output)
                    .status()
                    .expect("run cwebp");
                assert_eq!(status.code(), Some(0));
                let webp = fs::read(&output).expect("read the cwebp image");
                let payload = vp8_payload(&webp);
                let mut opaque = b"RIFF\0\0\0\0WEBPVP8 ".to_vec();
                opaque.extend_from_slice(&(payload.len() as u32).to_le_bytes());
                opaque.extend_from_slice(payload);
                assert_eq!(
                    read_quantizer_fields(&opaque),
                    (quantizer_index(quality), [0, 0, 0, -2, -4]),
                    "{} q {quality}",
                    fixture.name
                );
            }
        }
        fs::remove_dir_all(directory).expect("remove the test directory");
    }

    fn vp8_payload(webp: &[u8]) -> &[u8] {
        let mut position = 12;
        loop {
            let size =
                u32::from_le_bytes(webp[position + 4..position + 8].try_into().unwrap()) as usize;
            if &webp[position..position + 4] == b"VP8 " {
                return &webp[position + 8..position + 8 + size];
            }
            position += 8 + size + (size & 1);
        }
    }

    #[test]
    fn every_calibration_buffer_keeps_its_capacity_across_the_macroblock_loops() {
        let mut changes = Vec::new();
        for fixture in generator::all() {
            for quality in [50, 75, 90] {
                let encoded = encode(
                    &fixture.rgba,
                    fixture.width as usize,
                    fixture.height as usize,
                    4,
                    quantizer_index(quality),
                    &Options::default(),
                );
                for (buffer, (before, after)) in encoded.capacities.into_iter().enumerate() {
                    if before != after {
                        changes.push((fixture.name, quality, buffer, before, after));
                    }
                }
            }
        }
        assert_eq!(changes, []);
    }

    #[test]
    fn noise_and_the_largest_swept_dimensions_keep_token_capacity_at_quality_100() {
        for (name, width, height) in [
            ("noise", 64, 48),
            ("dimension-sweep", 48, 48),
            ("long-dimension-sweep", 16383, 1),
            ("long-dimension-sweep", 1, 16383),
            ("long-dimension-sweep", 16383, 3),
            ("long-dimension-sweep", 3, 16383),
            ("long-dimension-sweep", 4097, 1),
            ("long-dimension-sweep", 1, 4097),
        ] {
            let fixture = generator::noise(name, width, height);
            for bytes_per_pixel in [3, 4] {
                let pixels = if bytes_per_pixel == 3 {
                    rgb_bytes(&fixture.rgba)
                } else {
                    fixture.rgba.clone()
                };
                let encoded = encode(
                    &pixels,
                    width as usize,
                    height as usize,
                    bytes_per_pixel,
                    quantizer_index(100),
                    &Options::default(),
                );
                let (before, after) = encoded.capacities[0];
                assert_eq!(
                    before,
                    width.div_ceil(16) as usize * height.div_ceil(16) as usize * 801
                );
                assert_eq!(
                    after, before,
                    "{width}x{height}, {bytes_per_pixel} channels"
                );
            }
        }
    }

    #[test]
    fn the_header_round_trips_sent_probabilities_deltas_and_skip_probability() {
        let mut probabilities = crate::residual::DEFAULT_COEFF_PROBS;
        probabilities[0] = 1;
        probabilities[1055] = 255;
        let mut encoder = BoolEncoder::new();
        write_frame_header(&mut encoder, 26, Filter::Off, &probabilities, Some(64));
        let bytes = encoder.finish();
        let mut decoder = crate::bool_coder::BoolDecoder::new(&bytes);
        for width in [1, 1, 1, 1, 6, 3, 1, 2] {
            assert_eq!(decoder.read_literal(width), 0);
        }
        assert_eq!(decoder.read_literal(7), 26);
        assert_eq!(read_deltas(&mut decoder), [0, 0, 0, -2, -4]);
        assert_eq!(decoder.read_literal(1), 1);
        assert_eq!(read_updates(&mut decoder), probabilities);
        assert_eq!(decoder.read_literal(1), 1);
        assert_eq!(decoder.read_literal(8), 64);
    }

    #[test]
    fn the_partition_limit_drives_three_rungs_with_exact_reconstruction() {
        if !oracle_is_available("dwebp") {
            return;
        }
        let directory =
            scratch_directory("the_partition_limit_drives_three_rungs_with_exact_reconstruction");
        let fixtures = generator::all();
        let fixture = fixtures
            .iter()
            .find(|fixture| fixture.name == "diagonals")
            .unwrap();
        let mut lengths = [0; 3];
        let mut limit = 524287;
        for (rung, length) in lengths.iter_mut().enumerate() {
            let decision = super::ModeDecision {
                partition_limit: limit,
                ..super::ModeDecision::default()
            };
            let encoded = super::encode_with_decision(
                &fixture.rgba,
                fixture.width as usize,
                fixture.height as usize,
                4,
                quantizer_index(75),
                &Options {
                    filter: Filter::Off,
                    ..Options::default()
                },
                decision,
            );
            assert_eq!(usize::from(encoded.rung), rung);
            assert_encoded_reconstruction(&directory, fixture, quantizer_index(75), &encoded);
            let payload = vp8_payload(&encoded.webp);
            *length = (u32::from_le_bytes([payload[0], payload[1], payload[2], 0]) >> 5) as usize;
            limit = *length - 1;
        }
        assert_eq!(lengths, [137, 104, 104]);
        fs::remove_dir_all(directory).expect("remove the test directory");
    }
    #[test]
    fn flat_at_quality_75_has_top_left_dc_levels_and_zero_residuals_after_it() {
        let fixtures = generator::all();
        let fixture = fixtures
            .iter()
            .find(|fixture| fixture.name == "flat")
            .unwrap();
        let index = quantizer_index(75);
        assert_eq!(index, 26);
        assert_eq!(
            index - crate::quantize::QUANTIZER_DELTAS[3].unsigned_abs(),
            24
        );
        assert_eq!(factors(index).chroma_dc, 23);
        let source = crate::color::convert(&fixture.rgba, 32, 32, 4);
        let mut reconstruction = crate::color::YuvPlanes {
            y: vec![0; 1024],
            u: vec![0; 256],
            v: vec![0; 256],
            y_stride: 32,
            chroma_stride: 16,
        };
        let decision = super::ModeDecision::default();
        let mut residual_skips = [false; 4];
        for (position, skip) in residual_skips.iter_mut().enumerate() {
            let selected = decision.analyze(
                &source,
                &mut reconstruction,
                (position % 2, position / 2),
                factors(index),
            );
            let mut expected = crate::residual::MacroblockResidual::default();
            if position == 0 {
                assert_eq!((selected.luma_mode, selected.chroma_mode), (0, 1));
                expected.y2[0] = -9;
                for block in &mut expected.u {
                    block[0] = 7;
                }
                for block in &mut expected.v {
                    block[0] = -5;
                }
            }
            assert_eq!(selected.residual, expected, "macroblock {position}");
            *skip = !selected.residual.has_coefficients(true);
        }
        assert_eq!(residual_skips, [false, true, true, true]);
    }

    #[test]
    fn flat_at_quality_75_skips_the_three_macroblocks_after_the_top_left() {
        let fixtures = generator::all();
        let fixture = fixtures
            .iter()
            .find(|fixture| fixture.name == "flat")
            .unwrap();
        let encoded = encode(
            &fixture.rgba,
            32,
            32,
            4,
            quantizer_index(75),
            &Options::default(),
        );
        let (modes, skips, _) = read_frame_records(&encoded.webp, 2, 2);
        assert_eq!(modes[0], (0, 1, [0; 16]));
        assert_eq!(skips, [false, true, true, true]);
    }

    #[test]
    fn noise_at_quality_zero_codes_every_macroblock() {
        let fixtures = generator::all();
        let fixture = fixtures
            .iter()
            .find(|fixture| fixture.name == "noise")
            .unwrap();
        let encoded = encode(
            &fixture.rgba,
            64,
            48,
            4,
            quantizer_index(0),
            &Options::default(),
        );
        let (_, skips, probabilities) = read_frame_records(&encoded.webp, 4, 3);
        assert_eq!(skips, [false; 12]);
        assert_eq!(
            probabilities
                .iter()
                .zip(crate::residual::DEFAULT_COEFF_PROBS)
                .filter(|(a, b)| **a != *b)
                .count(),
            14
        );
    }
}
