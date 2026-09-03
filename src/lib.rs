//! Lossy WebP encoder for RGB and RGBA pixel buffers.
//!
//! The crate writes a single VP8 key frame inside a RIFF WebP container. It
//! reads and writes byte slices in memory, so the caller owns every path and
//! file handle. The library is `no_std` and allocates through `alloc`.
//!
//! Version 0.0.0 carries the public surface and the checks that guard it. A
//! call that clears the dimension and buffer checks returns
//! [`Error::Unimplemented`]. Encoding arrives in 0.1.0.
//!
//! [`Options`] is `#[non_exhaustive]`, so build one from its default and
//! assign the fields that change.
//!
//! ```
//! let mut opts = tiny_webp::Options::default();
//! opts.quality = 90;
//! opts.alpha = tiny_webp::Alpha::Discard;
//!
//! let err = tiny_webp::encode_rgba(&[], 0, 4, &opts).unwrap_err();
//! assert_eq!(
//!     err,
//!     tiny_webp::Error::DimensionsOutOfRange {
//!         width: 0,
//!         height: 4
//!     }
//! );
//! ```

#![no_std]
#![forbid(unsafe_code)]
#![warn(missing_docs)]
#![deny(clippy::float_arithmetic)]

extern crate alloc;
#[cfg(test)]
extern crate std;

#[allow(dead_code)]
mod bool_coder;
#[cfg_attr(not(test), allow(dead_code))]
pub(crate) mod prediction;
mod quantize;
#[allow(dead_code)]
mod residual;
#[cfg_attr(not(test), allow(dead_code))]
mod transform;

use alloc::vec::Vec;
use core::fmt;

/// Largest width or height a WebP bitstream carries, in pixels.
///
/// The VP8 key frame header spends 14 bits on each dimension, so each side
/// runs from 1 to this value.
pub const MAX_DIMENSION: u32 = 16383;

/// What the encoder does with the alpha plane of an RGBA input.
#[non_exhaustive]
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Alpha {
    /// Keep the plane and write it so it decodes byte for byte.
    #[default]
    Lossless,
    /// Drop the plane and write an opaque image.
    Discard,
}

/// The loop filter strength the encoder signals in the frame header.
///
/// A key frame encoder signals the filter and leaves the decoder to run it.
#[non_exhaustive]
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Filter {
    /// Derive the level and the sharpness from the quality setting.
    #[default]
    Auto,
    /// Signal an exact level and sharpness.
    Level {
        /// Filter level, 0 through 63.
        level: u8,
        /// Filter sharpness, 0 through 7.
        sharpness: u8,
    },
    /// Signal level 0, which leaves the decoder's filter idle.
    Off,
}

/// Settings for one encode.
///
/// [`Options::default`] matches cwebp's defaults for the knobs both tools
/// carry.
#[non_exhaustive]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Options {
    /// Quality from 0 to 100, on the scale cwebp's `-q` uses.
    pub quality: u8,
    /// Treatment of the alpha plane.
    pub alpha: Alpha,
    /// Write a `VP8X` container header even for an opaque image.
    ///
    /// An opaque image otherwise gets a bare `VP8 ` chunk. Set this for a
    /// consumer that wants one container shape for every output.
    pub force_vp8x: bool,
    /// Loop filter strength.
    pub filter: Filter,
}

impl Default for Options {
    fn default() -> Self {
        Self {
            quality: 75,
            alpha: Alpha::Lossless,
            force_vp8x: false,
            filter: Filter::Auto,
        }
    }
}

/// A call the encoder refused.
#[non_exhaustive]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Error {
    /// A width or a height outside 1 through [`MAX_DIMENSION`].
    DimensionsOutOfRange {
        /// The width the caller passed.
        width: u32,
        /// The height the caller passed.
        height: u32,
    },
    /// A pixel buffer whose length disagrees with the dimensions.
    BufferSizeMismatch {
        /// Bytes the dimensions ask for.
        expected: usize,
        /// Bytes the caller passed.
        actual: usize,
    },
    /// A well-formed call that 0.0.0 stops short of encoding.
    ///
    /// Every call that clears both checks returns this. The variant leaves
    /// the enum in 0.1.0, when encoding arrives.
    Unimplemented,
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::DimensionsOutOfRange { width, height } => {
                write!(
                    f,
                    "dimensions {width}x{height} fall outside 1..={MAX_DIMENSION}"
                )
            }
            Self::BufferSizeMismatch { expected, actual } => {
                write!(
                    f,
                    "buffer holds {actual} bytes and the dimensions ask for {expected}"
                )
            }
            Self::Unimplemented => {
                write!(
                    f,
                    "encoding is not implemented in {}",
                    env!("CARGO_PKG_VERSION")
                )
            }
        }
    }
}

impl core::error::Error for Error {}

/// Encodes an RGBA buffer into the bytes of a WebP file.
///
/// `rgba` holds `width * height` pixels in row order, four bytes each, in the
/// order red, green, blue, alpha.
///
/// # Errors
///
/// [`Error::DimensionsOutOfRange`] when a side falls outside 1 through
/// [`MAX_DIMENSION`], carrying the values passed. [`Error::BufferSizeMismatch`]
/// when `rgba` runs to a length other than `width * height * 4`. The dimension
/// check runs first, so a call that gets both wrong reports the dimensions.
///
/// [`Error::Unimplemented`] for every call that clears both checks.
pub fn encode_rgba(rgba: &[u8], width: u32, height: u32, opts: &Options) -> Result<Vec<u8>, Error> {
    // The encoder reads the options from 0.1.0.
    let _ = opts;
    check(rgba.len(), width, height, 4)?;
    Err(Error::Unimplemented)
}

/// Encodes an RGB buffer into the bytes of a WebP file.
///
/// `rgb` holds `width * height` pixels in row order, three bytes each, in the
/// order red, green, blue. The output is opaque, and [`Options::alpha`] has
/// nothing to act on.
///
/// # Errors
///
/// The same three errors as [`encode_rgba`], in the same order, with three
/// bytes per pixel in the length the buffer check asks for.
pub fn encode_rgb(rgb: &[u8], width: u32, height: u32, opts: &Options) -> Result<Vec<u8>, Error> {
    // The encoder reads the options from 0.1.0.
    let _ = opts;
    check(rgb.len(), width, height, 3)?;
    Err(Error::Unimplemented)
}

/// Holds the entry-point checks in one place so both spell the same order.
fn check(actual: usize, width: u32, height: u32, bytes_per_pixel: u64) -> Result<(), Error> {
    if width == 0 || height == 0 || width > MAX_DIMENSION || height > MAX_DIMENSION {
        return Err(Error::DimensionsOutOfRange { width, height });
    }
    // Past the guard both sides sit at 16383 or below, so the product fits in
    // 32 bits and the cast keeps every value.
    let expected = (u64::from(width) * u64::from(height) * bytes_per_pixel) as usize;
    if actual != expected {
        return Err(Error::BufferSizeMismatch { expected, actual });
    }
    Ok(())
}
