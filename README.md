# tiny-webp

Lossy WebP encoder in Rust, with a cwebp-shaped command line.

The encoder writes a single VP8 key frame inside a RIFF WebP container, from
the RGB or RGBA bytes a caller already holds. The library is `no_std` and
allocates through `alloc`. It works on byte slices in memory, so paths and
file handles stay with the caller.

## Status

Version 0.2.0 chooses among all VP8 luma, sub-block, and chroma prediction
modes by reconstruction error. It updates coefficient probabilities in two
passes, skips zero residuals, and applies chroma quantizer deltas. The default
loop filter level is the quantizer index capped at 63, with sharpness 0.
Opaque images use a bare `VP8 ` chunk. Transparent images add `VP8X` and an
uncompressed `ALPH` chunk that preserves every alpha byte.

[CALIBRATION.md](CALIBRATION.md) records size, RGB PSNR, speed, and memory at
`q` 50, 75, and 90 on a Mac Studio with an Apple M3 Ultra. For `photo-large`,
tiny-webp takes 2.188, 2.150, and 2.203 times cwebp's whole-process time at
those settings. Each ratio compares the median of five runs of each command
after one warm-up run. Every recorded row meets the memory bound of eight
bytes per pixel plus 1 MiB, including the caller's RGBA buffer.

At `q` 75, `photo-large` uses 1.499 times cwebp's bytes. Its RGB PSNR is
30.471 dB against cwebp's 29.515 dB. The size and PSNR relation varies by
fixture. These are the recorded comparisons with cwebp 1.6.0 at `q` 75:

| Fixture | Size divided by cwebp size | RGB PSNR, dB | cwebp RGB PSNR, dB |
|---|---:|---:|---:|
| `flat` | 0.784 | 45.121 | 47.599 |
| `checker` | 0.882 | 41.312 | 46.325 |
| `diagonals` | 1.378 | 44.757 | 39.189 |
| `gradient` | 1.518 | 40.930 | 41.104 |
| `text-blocks` | 1.258 | 40.307 | 37.751 |
| `noise` | 1.169 | 12.761 | 12.748 |
| `lowpass-noise` | 1.400 | 30.402 | 29.753 |
| `alpha-soft` | 4.810 | 40.930 | 40.037 |
| `alpha-hard` | 13.107 | 40.930 | 12.583 |
| `alpha-odd` | 2.564 | 40.374 | 35.699 |
| `photo-large` | 1.499 | 30.471 | 29.515 |
| `one-pixel` | 1.000 | 99.000 | 99.000 |
| `single-column` | 1.145 | 40.284 | 40.371 |
| `single-row` | 1.192 | 43.662 | 42.646 |
| `odd-size` | 1.587 | 40.070 | 35.853 |

The RGB PSNR test permits at most 3.0 dB below cwebp across the fixture set
at `q` 25, 50, 75, 90, and 95, with two exemptions. `flat` at `q` 50 is
4.39 dB below cwebp, and `checker` at `q` 75 is 5.01 dB below. Each exempted
pair's gap may exceed its recorded value by at most 0.1 dB. Four-segment
quantization remains the missing feature for both pairs.

## Input

The command reads PNG, JPEG, and still WebP from a path or stdin. Animated
WebP and CMYK JPEG inputs are unsupported. The command writes WebP to a path
or stdout.

## Install

Install the command:

```sh
cargo install tiny-webp
```

Add the library to `[dependencies]`:

```toml
tiny-webp = "0.2"
```

For the library alone:

```toml
tiny-webp = { version = "0.2", default-features = false, features = ["std"] }
```

## Library

`Options` is `#[non_exhaustive]`, so build one from its default and assign the
fields that change.

```rust
let mut opts = tiny_webp::Options::default();
opts.quality = 90;
opts.alpha = tiny_webp::Alpha::Discard;

let webp = tiny_webp::encode_rgba(&rgba, width, height, &opts)?;
```

`encode_rgb` takes the same arguments over three bytes per pixel and writes an
opaque image.

## Command line

```
usage: tiny-webp [options] <input> -o <output.webp>

  -q <0..100>, --quality <0..100>   quality, default 75
  -o <file>,   --output <file>      output path, or - for stdout
  -noalpha                          drop the alpha plane
  -f <0..100>                       loop filter strength, 0 turns it off
  -sharpness <0..7>                 loop filter sharpness, needs -f
  -print_psnr                       decode the result and print PSNR against the input
  -quiet                            no output on success
  -v                                print dimensions, bytes, and encode time
  -version, --version
  -h, --help

Filter strength S sets the level to round(S * 63 / 100).
```

cwebp spells `-noalpha`, `-quiet`, and `-version` with one dash, and both
spellings reach the same flag here. A bare `-` means stdin on the input and
stdout on the output.

A usage error exits 2 and puts the problem and this text on stderr. Any other
failure exits 1 with one line on stderr.

## License

Licensed under either of

- Apache License, Version 2.0 ([LICENSE-APACHE](LICENSE-APACHE))
- MIT license ([LICENSE-MIT](LICENSE-MIT))

at your option.
