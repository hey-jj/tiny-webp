# Changelog

All notable changes to this project are documented here. The format follows
Keep a Changelog, and the project uses semantic versioning.

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

The charter tree. Nothing at this version is published. A library call that
clears the dimension and buffer checks returns `Error::Unimplemented`, and the
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
  version of the oracle that later milestones measure against.
