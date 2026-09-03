//! What the two entry points return for well-formed and malformed calls.

use tiny_webp::{encode_rgb, encode_rgba, Error, Options};

/// Builds options at a quality the way a caller outside the crate builds them.
fn at_quality(quality: u8) -> Options {
    let mut opts = Options::default();
    opts.quality = quality;
    opts
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
