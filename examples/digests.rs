//! Prints SHA-256 digests for every fixed fixture encode.

#[path = "../fixtures/generator.rs"]
mod generator;

use std::fmt::Write;

use sha2::{Digest, Sha256};
use tiny_webp::{Alpha, Options};

struct Row {
    fixture: &'static str,
    quality: u8,
    entry: &'static str,
    options: &'static str,
    digest: [u8; 32],
}

pub(crate) fn manifest() -> String {
    let mut rows = Vec::new();

    for fixture in generator::all() {
        let rgb: Vec<u8> = fixture
            .rgba
            .chunks_exact(4)
            .flat_map(|pixel| pixel[..3].iter().copied())
            .collect();
        for quality in 0..=100 {
            if fixture.name == "photo-large" && ![0, 25, 50, 75, 90, 95, 100].contains(&quality) {
                continue;
            }
            let mut options = Options::default();
            options.quality = quality;

            rows.push(Row {
                fixture: fixture.name,
                quality,
                entry: "encode_rgb",
                options: "default",
                digest: hash(
                    &tiny_webp::encode_rgb(&rgb, fixture.width, fixture.height, &options)
                        .expect("the fixture dimensions and RGB length agree"),
                ),
            });
            rows.push(Row {
                fixture: fixture.name,
                quality,
                entry: "encode_rgba",
                options: "default",
                digest: hash(
                    &tiny_webp::encode_rgba(&fixture.rgba, fixture.width, fixture.height, &options)
                        .expect("the fixture dimensions and RGBA length agree"),
                ),
            });
        }

        let mut force_vp8x = Options::default();
        force_vp8x.quality = 75;
        force_vp8x.force_vp8x = true;
        rows.push(Row {
            fixture: fixture.name,
            quality: 75,
            entry: "encode_rgba",
            options: "force_vp8x",
            digest: hash(
                &tiny_webp::encode_rgba(&fixture.rgba, fixture.width, fixture.height, &force_vp8x)
                    .expect("the fixture dimensions and RGBA length agree"),
            ),
        });

        let mut alpha_discard = Options::default();
        alpha_discard.quality = 75;
        alpha_discard.alpha = Alpha::Discard;
        rows.push(Row {
            fixture: fixture.name,
            quality: 75,
            entry: "encode_rgba",
            options: "alpha_discard",
            digest: hash(
                &tiny_webp::encode_rgba(
                    &fixture.rgba,
                    fixture.width,
                    fixture.height,
                    &alpha_discard,
                )
                .expect("the fixture dimensions and RGBA length agree"),
            ),
        });
    }

    rows.sort_by_key(|row| (row.fixture, row.quality, row.entry, row.options));

    let mut output = String::new();
    for row in rows {
        write!(
            output,
            "{} {} {} {} ",
            row.fixture, row.quality, row.entry, row.options
        )
        .expect("writing to a string succeeds");
        for byte in row.digest {
            write!(output, "{byte:02x}").expect("writing to a string succeeds");
        }
        output.push('\n');
    }
    output
}

fn hash(bytes: &[u8]) -> [u8; 32] {
    Sha256::digest(bytes).into()
}

#[cfg(not(test))]
fn main() {
    print!("{}", manifest());
}
