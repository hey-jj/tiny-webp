# tiny-webp

Lossy WebP encoder in Rust, with a cwebp-shaped command line.

The encoder writes a single VP8 key frame inside a RIFF WebP container, from
the RGB or RGBA bytes a caller already holds. The library is `no_std` and
allocates through `alloc`. It works on byte slices in memory, so paths and
file handles stay with the caller.

## Status

Version 0.0.0 is the charter tree. It carries the public surface, the checks
that guard it, the fixture generator, the benchmark harness, and a CI workflow
that runs on Linux and macOS. Every library call that clears the dimension and
buffer checks returns `Error::Unimplemented`. The binary reads a command line
and prints its usage. Encoding arrives in 0.1.0.

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
  -quiet                            no output on success
  -v                                print dimensions, bytes, and encode time
  -version, --version
  -h, --help
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
