//! Fixture images, generated from named formulas.
//!
//! Every value comes from integer arithmetic and from a generator seeded by
//! the fixture name, so the bytes are the same on every target and on every
//! run. The examples, the benchmark, and the tests all compile this file.

use std::vec;
use std::vec::Vec;

/// One generated image, in RGBA order at four bytes per pixel.
pub struct Fixture {
    /// The name the formula is seeded from and the reports print.
    pub name: &'static str,
    /// Width in pixels.
    pub width: u32,
    /// Height in pixels.
    pub height: u32,
    /// `width * height * 4` bytes in row order.
    pub rgba: Vec<u8>,
}

/// Builds the whole fixture table.
///
/// The table covers a gradient, a hard-edged shape, a noise field, a low-pass
/// noise field, a soft alpha mask, a hard alpha mask, and the sizes 1x1, 1x33,
/// 33x1, and 17x31.
pub fn all() -> Vec<Fixture> {
    vec![
        flat("flat", 32, 32),
        checker("checker", 32, 32),
        gradient("gradient", 64, 48),
        text_blocks("text-blocks", 64, 48),
        noise("noise", 64, 48),
        lowpass_noise("lowpass-noise", 64, 48),
        soft_alpha("alpha-soft", 64, 48),
        hard_alpha("alpha-hard", 64, 48),
        soft_alpha("alpha-odd", 17, 31),
        lowpass_noise("photo-large", 1024, 768),
        gradient("one-pixel", 1, 1),
        gradient("single-column", 1, 33),
        gradient("single-row", 33, 1),
        text_blocks("odd-size", 17, 31),
    ]
}

/// One opaque color across the whole image.
fn flat(name: &'static str, width: u32, height: u32) -> Fixture {
    let mut rgba = Vec::with_capacity(pixel_bytes(width, height));
    for _ in 0..width * height {
        rgba.extend_from_slice(&[96, 128, 160, 255]);
    }
    Fixture {
        name,
        width,
        height,
        rgba,
    }
}

/// Black and white squares whose sides span two pixels.
fn checker(name: &'static str, width: u32, height: u32) -> Fixture {
    let mut rgba = Vec::with_capacity(pixel_bytes(width, height));
    for y in 0..height {
        for x in 0..width {
            let value = if (x / 2 + y / 2) % 2 == 0 { 0 } else { 255 };
            rgba.extend_from_slice(&[value, value, value, 255]);
        }
    }
    Fixture {
        name,
        width,
        height,
        rgba,
    }
}

/// A linear ramp on each axis with a full alpha plane.
fn gradient(name: &'static str, width: u32, height: u32) -> Fixture {
    let mut rgba = Vec::with_capacity(pixel_bytes(width, height));
    for y in 0..height {
        for x in 0..width {
            rgba.push(ramp(x, width));
            rgba.push(ramp(y, height));
            rgba.push(ramp(x + y, width + height - 1));
            rgba.push(255);
        }
    }
    Fixture {
        name,
        width,
        height,
        rgba,
    }
}

/// Dark strokes on a light ground, with edges that land on pixel boundaries.
fn text_blocks(name: &'static str, width: u32, height: u32) -> Fixture {
    let mut rgba = Vec::with_capacity(pixel_bytes(width, height));
    for y in 0..height {
        for x in 0..width {
            let stem = x % 7 < 2 && y % 11 < 8;
            let bar = y % 11 == 8 && x % 14 < 9;
            let value = if stem || bar { 24 } else { 240 };
            rgba.push(value);
            rgba.push(value);
            rgba.push(value);
            rgba.push(255);
        }
    }
    Fixture {
        name,
        width,
        height,
        rgba,
    }
}

/// An independent value in every color byte.
fn noise(name: &'static str, width: u32, height: u32) -> Fixture {
    let mut rng = Rng::seeded(name);
    let mut rgba = Vec::with_capacity(pixel_bytes(width, height));
    for _ in 0..width * height {
        rgba.push(rng.next_byte());
        rgba.push(rng.next_byte());
        rgba.push(rng.next_byte());
        rgba.push(255);
    }
    Fixture {
        name,
        width,
        height,
        rgba,
    }
}

/// Noise run through two box blurs, which leaves the soft gradients a
/// photograph carries.
fn lowpass_noise(name: &'static str, width: u32, height: u32) -> Fixture {
    let mut rng = Rng::seeded(name);
    let count = (width * height) as usize;
    let mut planes: Vec<Vec<u8>> = (0..3)
        .map(|_| (0..count).map(|_| rng.next_byte()).collect())
        .collect();
    for plane in &mut planes {
        *plane = blur(plane, width, height);
        *plane = blur(plane, width, height);
    }

    let mut rgba = Vec::with_capacity(pixel_bytes(width, height));
    for ((red, green), blue) in planes[0].iter().zip(&planes[1]).zip(&planes[2]) {
        rgba.push(*red);
        rgba.push(*green);
        rgba.push(*blue);
        rgba.push(255);
    }
    Fixture {
        name,
        width,
        height,
        rgba,
    }
}

/// A gradient under an alpha plane that falls off from the center.
fn soft_alpha(name: &'static str, width: u32, height: u32) -> Fixture {
    let mut fixture = gradient(name, width, height);
    // Coordinates are doubled so the center of an even-sided image lands on a
    // whole number.
    let span_x = i64::from(width) - 1;
    let span_y = i64::from(height) - 1;
    let corner = span_x * span_x + span_y * span_y;
    for y in 0..height {
        for x in 0..width {
            let dx = 2 * i64::from(x) - span_x;
            let dy = 2 * i64::from(y) - span_y;
            let alpha = if corner == 0 {
                255
            } else {
                255 - ((dx * dx + dy * dy) * 255 / corner).min(255)
            };
            let index = (y as usize * width as usize + x as usize) * 4 + 3;
            fixture.rgba[index] = alpha as u8;
        }
    }
    fixture
}

/// A gradient under an alpha plane that steps between 0 and 255.
fn hard_alpha(name: &'static str, width: u32, height: u32) -> Fixture {
    let mut fixture = gradient(name, width, height);
    for y in 0..height {
        for x in 0..width {
            let inside =
                x * 4 >= width && x * 4 < width * 3 && y * 4 >= height && y * 4 < height * 3;
            let index = (y as usize * width as usize + x as usize) * 4 + 3;
            fixture.rgba[index] = if inside { 255 } else { 0 };
        }
    }
    fixture
}

/// Averages each sample with its eight neighbors, clamping at the edges.
fn blur(plane: &[u8], width: u32, height: u32) -> Vec<u8> {
    let mut out = Vec::with_capacity(plane.len());
    let last_x = i64::from(width) - 1;
    let last_y = i64::from(height) - 1;
    for y in 0..height {
        for x in 0..width {
            let mut sum: u32 = 0;
            for dy in -1..=1_i64 {
                for dx in -1..=1_i64 {
                    let sx = (i64::from(x) + dx).clamp(0, last_x) as usize;
                    let sy = (i64::from(y) + dy).clamp(0, last_y) as usize;
                    sum += u32::from(plane[sy * width as usize + sx]);
                }
            }
            out.push((sum / 9) as u8);
        }
    }
    out
}

/// Spreads `index` over 0 to 255 across `count` steps.
fn ramp(index: u32, count: u32) -> u8 {
    if count <= 1 {
        0
    } else {
        (index * 255 / (count - 1)) as u8
    }
}

/// The RGBA byte length of an image.
fn pixel_bytes(width: u32, height: u32) -> usize {
    width as usize * height as usize * 4
}

/// A 32-bit generator whose sequence follows from a fixture name.
struct Rng(u32);

impl Rng {
    /// Seeds the generator with the FNV-1a hash of `name`.
    fn seeded(name: &str) -> Self {
        let mut state: u32 = 0x811c_9dc5;
        for byte in name.as_bytes() {
            state ^= u32::from(*byte);
            state = state.wrapping_mul(0x0100_0193);
        }
        // xorshift stalls at zero, so the low bit is forced on.
        Self(state | 1)
    }

    /// Advances the xorshift32 state and returns it.
    fn next_u32(&mut self) -> u32 {
        let mut state = self.0;
        state ^= state << 13;
        state ^= state >> 17;
        state ^= state << 5;
        self.0 = state;
        state
    }

    /// Takes the top byte of the next word, which mixes better than the low one.
    fn next_byte(&mut self) -> u8 {
        (self.next_u32() >> 24) as u8
    }
}
