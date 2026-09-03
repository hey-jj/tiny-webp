//! What the two entry points return for well-formed and malformed calls.

use tiny_webp::{encode_rgb, encode_rgba, Error, Options, MAX_DIMENSION};

/// Builds options at a quality the way a caller outside the crate builds them.
fn at_quality(quality: u8) -> Options {
    let mut opts = Options::default();
    opts.quality = quality;
    opts
}

#[test]
fn every_size_from_one_to_forty_eight_writes_a_riff_file_at_three_qualities() {
    for quality in [0u8, 50, 100] {
        let opts = at_quality(quality);
        for width in 1..=48u32 {
            for height in 1..=48u32 {
                let pixels = width as usize * height as usize;
                let rgba = encode_rgba(&vec![0u8; pixels * 4], width, height, &opts)
                    .map(|bytes| bytes[..4].to_vec());
                let rgb = encode_rgb(&vec![0u8; pixels * 3], width, height, &opts)
                    .map(|bytes| bytes[..4].to_vec());
                assert_eq!(
                    rgba,
                    Ok(b"RIFF".to_vec()),
                    "RGBA {width}x{height} q{quality}"
                );
                assert_eq!(rgb, Ok(b"RIFF".to_vec()), "RGB {width}x{height} q{quality}");
            }
        }
    }
}

#[test]
fn a_zero_or_oversized_side_comes_back_carrying_the_values_that_were_passed() {
    let opts = Options::default();
    for (width, height) in [
        (0u32, 8u32),
        (8, 0),
        (0, 0),
        (16384, 8),
        (8, 16384),
        (16384, 16384),
    ] {
        assert_eq!(
            encode_rgba(&[], width, height, &opts),
            Err(Error::DimensionsOutOfRange { width, height })
        );
        assert_eq!(
            encode_rgb(&[], width, height, &opts),
            Err(Error::DimensionsOutOfRange { width, height })
        );
    }
}

#[test]
fn the_longest_side_the_bitstream_carries_writes_a_riff_file_on_either_axis() {
    let opts = Options::default();
    let strip = vec![0u8; MAX_DIMENSION as usize * 4];
    let horizontal = encode_rgba(&strip, MAX_DIMENSION, 1, &opts).map(|bytes| bytes[..4].to_vec());
    let vertical = encode_rgba(&strip, 1, MAX_DIMENSION, &opts).map(|bytes| bytes[..4].to_vec());
    assert_eq!(horizontal, Ok(b"RIFF".to_vec()));
    assert_eq!(vertical, Ok(b"RIFF".to_vec()));
}

#[test]
fn a_buffer_one_byte_short_or_one_byte_long_reports_both_counts() {
    let opts = Options::default();

    let rgba_expected = 4 * 5 * 4;
    for actual in [rgba_expected - 1, rgba_expected + 1] {
        assert_eq!(
            encode_rgba(&vec![0u8; actual], 4, 5, &opts),
            Err(Error::BufferSizeMismatch {
                expected: rgba_expected,
                actual
            })
        );
    }

    let rgb_expected = 4 * 5 * 3;
    for actual in [rgb_expected - 1, rgb_expected + 1] {
        assert_eq!(
            encode_rgb(&vec![0u8; actual], 4, 5, &opts),
            Err(Error::BufferSizeMismatch {
                expected: rgb_expected,
                actual
            })
        );
    }
}

#[test]
fn a_call_that_gets_the_dimensions_and_the_buffer_wrong_reports_the_dimensions() {
    let opts = Options::default();
    assert_eq!(
        encode_rgba(&[1, 2, 3], 0, 16384, &opts),
        Err(Error::DimensionsOutOfRange {
            width: 0,
            height: 16384
        })
    );
    assert_eq!(
        encode_rgb(&[1, 2, 3], 0, 16384, &opts),
        Err(Error::DimensionsOutOfRange {
            width: 0,
            height: 16384
        })
    );
}

#[test]
fn quality_above_one_hundred_produces_the_same_bytes_as_one_hundred() {
    let pixels = [64u8; 4 * 4 * 4];
    let at_one_hundred = at_quality(100);
    let above_one_hundred = at_quality(255);
    assert_eq!(
        encode_rgba(&pixels, 4, 4, &above_one_hundred),
        encode_rgba(&pixels, 4, 4, &at_one_hundred)
    );
}
