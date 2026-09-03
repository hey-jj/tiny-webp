#![no_main]

use std::io::Cursor;

use image_webp::WebPDecoder;
use libfuzzer_sys::fuzz_target;
use tiny_webp::{encode_rgb, encode_rgba, Alpha, Error, Options};

fuzz_target!(|data: &[u8]| {
    if data.len() < 6 {
        return;
    }

    let width = usize::from(u16::from_le_bytes([data[0], data[1]])) % 512 + 1;
    let height = usize::from(u16::from_le_bytes([data[2], data[3]])) % 512 + 1;
    let quality = data[4];
    let option_bits = data[5];
    let use_rgba = option_bits & 1 != 0;
    let force_opaque = option_bits & 2 != 0;

    let mut options = Options::default();
    options.quality = quality;
    if option_bits & 4 != 0 {
        options.alpha = Alpha::Discard;
    }
    options.force_vp8x = option_bits & 8 != 0;

    let bytes_per_pixel = if use_rgba { 4 } else { 3 };
    let expected = width * height * bytes_per_pixel;
    let available = &data[6..];
    let used = available.len().min(expected);
    let mut opaque_pixels = Vec::new();
    let pixels = if use_rgba && force_opaque {
        opaque_pixels.extend_from_slice(&available[..used]);
        for alpha in opaque_pixels.iter_mut().skip(3).step_by(4) {
            *alpha = 255;
        }
        opaque_pixels.as_slice()
    } else {
        &available[..used]
    };

    let encoded = if use_rgba {
        encode_rgba(pixels, width as u32, height as u32, &options)
    } else {
        encode_rgb(pixels, width as u32, height as u32, &options)
    };

    if available.len() < expected {
        assert_eq!(
            encoded,
            Err(Error::BufferSizeMismatch {
                expected,
                actual: available.len()
            })
        );
        return;
    }

    let webp = encoded.expect("a complete pixel buffer must encode");
    let mut decoder = WebPDecoder::new(Cursor::new(webp)).expect("the encoded WebP must decode");
    assert_eq!(decoder.dimensions(), (width as u32, height as u32));
    let channels = if decoder.has_alpha() { 4 } else { 3 };
    let mut decoded = vec![0; width * height * channels];
    assert_eq!(u8::from(decoder.read_image(&mut decoded).is_ok()), 1);
});
