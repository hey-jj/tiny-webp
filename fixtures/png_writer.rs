//! Writes eight-bit PNG pixel buffers for examples and test support.

use std::io::Write;

/// Writes one eight-bit PNG image.
pub fn write<W: Write>(
    output: W,
    width: u32,
    height: u32,
    color: png::ColorType,
    pixels: &[u8],
) -> Result<(), png::EncodingError> {
    let mut encoder = png::Encoder::new(output, width, height);
    encoder.set_color(color);
    encoder.set_depth(png::BitDepth::Eight);
    let mut writer = encoder.write_header()?;
    writer.write_image_data(pixels)
}
