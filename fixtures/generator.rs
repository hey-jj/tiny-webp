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
/// Includes `flat`, `checker`, `diagonals`, `gradient`, `text-blocks`,
/// `noise`, `lowpass-noise`, `alpha-soft`, `alpha-hard`, `alpha-odd`,
/// `photo-large`, `one-pixel`, `single-column`, `single-row`, and `odd-size`.
pub fn all() -> Vec<Fixture> {
    vec![
        flat("flat", 32, 32),
        checker("checker", 32, 32),
        diagonals(),
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

/// Four opaque regions with edges at slopes 1, -1, 1/2, and -1/2.
fn diagonals() -> Fixture {
    let mut rgba = Vec::with_capacity(48 * 48 * 4);
    for y in 0..48 {
        for x in 0..48 {
            let local_x = x % 24;
            let local_y = y % 24;
            let bright = match (x / 24, y / 24) {
                (0, 0) => local_y >= local_x,
                (1, 0) => local_y + local_x >= 23,
                (0, 1) => 2 * local_y >= local_x + 12,
                _ => 2 * local_y + local_x >= 35,
            };
            let value = if bright { 240 } else { 24 };
            rgba.extend_from_slice(&[value, value, value, 255]);
        }
    }
    Fixture {
        name: "diagonals",
        width: 48,
        height: 48,
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
pub(crate) fn noise(name: &'static str, width: u32, height: u32) -> Fixture {
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

/// A 32-bit generator whose sequence follows from a name.
pub(crate) struct Rng(u32);

impl Rng {
    /// Seeds the generator with the FNV-1a hash of `name`.
    pub(crate) fn seeded(name: &str) -> Self {
        let mut state: u32 = 0x811c_9dc5;
        for byte in name.as_bytes() {
            state ^= u32::from(*byte);
            state = state.wrapping_mul(0x0100_0193);
        }
        // x ^ (x << n) and x ^ (x >> n) both keep zero at zero.
        Self(state | 1)
    }

    /// Advances the xorshift32 state and returns it.
    pub(crate) fn next_u32(&mut self) -> u32 {
        let mut state = self.0;
        state ^= state << 13;
        state ^= state >> 17;
        state ^= state << 5;
        self.0 = state;
        state
    }

    /// Returns the top byte of the next word, `word >> 24`.
    pub(crate) fn next_byte(&mut self) -> u8 {
        self.below(256) as u8
    }

    /// Returns `floor(word * bound / 2^32)` from the next word.
    ///
    /// # Panics
    ///
    /// Panics when `bound` is zero.
    pub(crate) fn below(&mut self, bound: u32) -> u32 {
        assert!(bound > 0, "the random bound must be positive");
        ((u64::from(self.next_u32()) * u64::from(bound)) >> 32) as u32
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn sources_with_the_same_name_agree_for_one_thousand_draws() {
        for name in ["", "random-draws", "noise", "pixels-\u{03bb}"] {
            let mut left = super::Rng::seeded(name);
            let mut right = super::Rng::seeded(name);
            for _ in 0..1_000 {
                assert_eq!(left.next_u32(), right.next_u32());
                assert_eq!(left.next_byte(), right.next_byte());
                assert_eq!(left.below(65_536), right.below(65_536));
            }
        }
    }

    #[test]
    fn different_names_differ_within_sixteen_draws() {
        let mut left = super::Rng::seeded("random-draws");
        let mut right = super::Rng::seeded("bounded-draws");
        let words: [(u32, u32); 16] = core::array::from_fn(|_| (left.next_u32(), right.next_u32()));
        assert_eq!(words.iter().position(|(a, b)| a != b), Some(0));
    }

    #[test]
    fn named_draws_pin_words_bytes_and_bounded_values() {
        let mut rng = super::Rng::seeded("random-draws");
        let draws: [(u32, u8, u32); 4] =
            core::array::from_fn(|_| (rng.next_u32(), rng.next_byte(), rng.below(65_536)));
        assert_eq!(
            draws,
            [
                (2_726_729_151, 118, 20_277),
                (1_135_808_518, 23, 63_789),
                (3_920_657_851, 38, 19_053),
                (486_381_369, 109, 33_687),
            ]
        );
    }

    #[test]
    fn ten_thousand_bounded_draws_stay_below_each_bound() {
        for (bound, expected_min, expected_max, expected_sum) in [
            (1, 0, 0, 0),
            (2, 0, 1, 5_096),
            (3, 0, 2, 10_184),
            (255, 0, 254, 1_286_897),
            (65_536, 2, 65_533, 332_015_601),
            (u32::MAX, 167_964, 4_294_801_605, 21_759_301_293_698),
        ] {
            let mut rng = super::Rng::seeded("bounded-draws");
            let mut minimum = u32::MAX;
            let mut maximum = 0;
            let mut sum = 0_u64;
            let mut out_of_range = 0;
            for _ in 0..10_000 {
                let value = rng.below(bound);
                minimum = minimum.min(value);
                maximum = maximum.max(value);
                sum += u64::from(value);
                out_of_range += usize::from(value >= bound);
            }
            assert_eq!(out_of_range, 0, "bound {bound}");
            assert_eq!(
                (minimum, maximum, sum),
                (expected_min, expected_max, expected_sum)
            );
        }
    }

    #[test]
    fn a_zero_bound_panics_before_advancing_the_source() {
        let mut rng = super::Rng::seeded("random-draws");
        let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| rng.below(0)));
        let error = result.expect_err("a zero bound must panic");
        assert_eq!(
            error.downcast_ref::<&str>(),
            Some(&"the random bound must be positive")
        );
        assert_eq!(rng.next_u32(), 2_726_729_151);
    }
}
