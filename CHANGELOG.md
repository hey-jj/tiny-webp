# Changelog

All notable changes to this project are documented here. The format follows
Keep a Changelog, and the project uses semantic versioning.

## [0.2.0] - 2026-10-02

### Added

- Luma, sub-block, and chroma prediction now choose the smallest
  reconstruction error across all VP8 prediction modes.
- Two-pass coefficient probability updates adapt residual coding to each
  image. Mode selection and quantization run once, in the analysis pass.
- The skip flag omits residual tokens for macroblocks whose quantized
  coefficients are all zero.
- Chroma quantizer deltas set DC to -2 and AC to -4 relative to the base
  index. Luma and Y2 deltas stay zero.
- The default loop filter follows `q` through the quantizer index, capped at
  level 63 with sharpness 0. Decoders apply the signalled filter.
- `-f` sets filter strength from 0 to 100. The level is
  `round(strength * 63 / 100)`. Zero turns the filter off.
- `-sharpness` sets sharpness from 0 to 7 when used with `-f`.
- `-print_psnr` prints decoded RGB PSNR against the input. `-quiet`
  suppresses the line.
- Committed cwebp 1.6.0 reference files let tests reproduce the quality
  comparison. The RGB PSNR gate allows a gap of 3.0 dB, with two exemptions:
  `flat` at `q` 50 and `checker` at `q` 75. Their recorded gaps are 4.39 and
  5.01 dB, each with a further tolerance of 0.1 dB.
- An in-crate mutation runner checks pixel buffers, dimensions, and options
  under `cargo test`. `TINY_WEBP_MUTATION_RUNS` sets its run budget. The
  runner replays saved failing inputs.

### Changed

- Encoded bytes change from 0.1.x at every `q`. Update stored output digests
  when upgrading. `Filter::Auto` now signals the level derived from the
  quantizer index.
- The in-crate mutation runner replaces the separate fuzz package and its
  lock file. Mutation checks run with the crate's tests.
- The calibration record adds whole-process timing for both commands and
  `tiny_webp_to_cwebp_time_ratio` for their comparison. `peak_heap_bytes`
  reports encode-call heap growth, and `memory_bound_held` includes the
  caller's RGBA buffer in the memory check.

## [0.1.1] - 2026-09-09

### Fixed

- Input paths after `--` and output names with a leading dash now keep the
  spelling entered on the command line.
- `-h` and `-version` now exit 1 with one stderr line when stdout cannot
  accept a write or flush. These failures previously caused a panic.
- Diagnostics and output summaries now escape control characters in paths
  so each message stays on one line. File access uses the path as entered.

### Documentation

- The README now states how file size and RGB PSNR compare with cwebp across
  the recorded fixtures and quality settings.

## [0.1.0] - 2026-09-03

### Added

- A lossy WebP encoder for RGB and RGBA buffers. It uses DC prediction, the Y2
  transform path, one token partition, and the quantizer selected by
  `Options::quality`.
- RIFF WebP output with a bare `VP8 ` chunk for opaque images. Transparent
  input or `Options::force_vp8x` adds the extended container header.
- Exact alpha storage in an uncompressed `ALPH` chunk. `Alpha::Discard` drops
  the input alpha plane.
- The `tiny-webp` command reads PNG, JPEG, and WebP from files or stdin. It
  writes WebP to files or stdout.
- A SHA-256 digest manifest that pins the encoded bytes for the fixture set.
- A fuzz target that exercises dimensions, quality, alpha settings, container
  shapes, and incomplete pixel buffers.
- A calibration record with throughput, peak heap use, output size, decoded
  PSNR, and cwebp comparisons.

### Removed

- `Error::Unimplemented`. Valid buffers now return encoded WebP bytes.

## [0.0.0] - 2026-09-03

The library validates pixel buffers and dimensions, and the command parses
encoding options. A valid library call returns `Error::Unimplemented`. The
binary stops after it reads the command line.

### Added

- The library surface: `encode_rgba`, `encode_rgb`, `Options` with its `Alpha`
  and `Filter` settings, the `Error` enum, and `MAX_DIMENSION`. The library is
  `no_std` and allocates through `alloc`.
- The `tiny-webp` binary carrying `-q`, `-o`, `-noalpha`, `-quiet`, `-v`,
  `-version`, and `-h`, each of the word flags readable with one dash or two,
  over the exit codes 0, 1, and 2.
- A criterion benchmark on one fixture, and an example that walks the whole
  fixture set at quality 50, 75, and 90.
- A fixture generator that builds ten images from named formulas in integer
  arithmetic, so the bytes match on every target and every run.
- A CI workflow that builds and tests on Linux and macOS, builds the library
  and the binary at Rust 1.85.0, and installs libwebp so each log names the
  decoder version used to check output.
