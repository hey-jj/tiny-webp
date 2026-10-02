//! The committed digest manifest matches every fixture encode.

#![forbid(unsafe_code)]

#[path = "../examples/digests.rs"]
mod digests;

use digests::generator;

use std::io::Cursor;
use std::process::Command;
use std::sync::OnceLock;

use sha2::{Digest, Sha256};
use tiny_webp::{Alpha, Options};

fn generated_manifest() -> &'static str {
    static MANIFEST: OnceLock<String> = OnceLock::new();
    MANIFEST.get_or_init(digests::manifest)
}

#[test]
fn regenerating_the_digest_manifest_matches_the_committed_text() {
    let path = concat!(env!("CARGO_MANIFEST_DIR"), "/fixtures/digests.txt");
    let committed = std::fs::read_to_string(path).expect("read the committed digest manifest");
    assert_eq!(generated_manifest(), committed);
}

#[test]
fn the_digest_manifest_contains_each_fixed_encode_in_sorted_order() {
    let actual: Vec<(&str, u8, &str, &str)> = generated_manifest()
        .lines()
        .map(|line| {
            let fields: Vec<&str> = line.split_whitespace().collect();
            assert_eq!(fields.len(), 5, "{line}");
            assert_eq!(fields[4].len(), 64, "{line}");
            assert!(
                fields[4]
                    .bytes()
                    .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte)),
                "{line}"
            );
            (
                fields[0],
                fields[1].parse().expect("quality is an integer"),
                fields[2],
                fields[3],
            )
        })
        .collect();

    let mut expected = Vec::new();
    for fixture in [
        "alpha-hard",
        "alpha-odd",
        "alpha-soft",
        "checker",
        "diagonals",
        "flat",
        "gradient",
        "lowpass-noise",
        "noise",
        "odd-size",
        "one-pixel",
        "photo-large",
        "single-column",
        "single-row",
        "text-blocks",
    ] {
        for quality in 0..=100 {
            if fixture == "photo-large" && ![0, 25, 50, 75, 90, 95, 100].contains(&quality) {
                continue;
            }
            expected.push((fixture, quality, "encode_rgb", "default"));
            expected.push((fixture, quality, "encode_rgba", "default"));
        }
        expected.push((fixture, 75, "encode_rgba", "alpha_discard"));
        expected.push((fixture, 75, "encode_rgba", "force_vp8x"));
    }
    expected.sort_unstable_by_key(|(fixture, quality, entry, options)| {
        format!("{fixture} {quality} {entry} {options}")
    });

    assert_eq!(actual.len(), 2872);
    assert_eq!(actual, expected);
}

#[test]
fn forced_extended_rows_match_default_rows_for_transparent_fixtures() {
    for fixture in ["alpha-hard", "alpha-odd", "alpha-soft"] {
        let default = digest_for(fixture, "default");
        let forced = digest_for(fixture, "force_vp8x");
        assert_eq!(forced, default, "{fixture}");
    }
}

fn digest_for(fixture: &str, options: &str) -> &'static str {
    generated_manifest()
        .lines()
        .find_map(|line| {
            let mut fields = line.split_whitespace();
            let name = fields.next()?;
            let quality = fields.next()?;
            let entry = fields.next()?;
            let row_options = fields.next()?;
            let digest = fields.next()?;
            (name == fixture && quality == "75" && entry == "encode_rgba" && row_options == options)
                .then_some(digest)
        })
        .expect("the manifest contains the requested row")
}

#[test]
fn every_manifest_row_decodes_at_its_input_dimensions_and_preserves_its_alpha() {
    let oracle_available = Command::new("dwebp")
        .arg("-version")
        .output()
        .map(|output| output.status.success())
        .unwrap_or(false);
    if std::env::var("TINY_WEBP_REQUIRE_ORACLE").as_deref() == Ok("1") {
        assert!(oracle_available, "dwebp is required");
    }
    let directory = std::env::temp_dir().join(format!(
        "tiny-webp-every_manifest_row_decodes_at_its_input_dimensions_and_preserves_its_alpha-{}",
        std::process::id()
    ));
    if directory.exists() {
        std::fs::remove_dir_all(&directory).expect("remove the old test directory");
    }
    std::fs::create_dir_all(&directory).expect("create the test directory");
    let input = directory.join("input.webp");
    let output = directory.join("output.png");
    let fixtures = generator::all();
    let mut rows = 0;
    let mut alpha_rows = 0;
    for line in include_str!("../fixtures/digests.txt").lines() {
        let fields: Vec<_> = line.split_whitespace().collect();
        assert_eq!(fields.len(), 5, "{line}");
        let fixture = fixtures
            .iter()
            .find(|fixture| fixture.name == fields[0])
            .expect("find the manifest fixture");
        let mut options = Options::default();
        options.quality = fields[1].parse().expect("read the manifest quality");
        match fields[3] {
            "default" => (),
            "alpha_discard" => options.alpha = Alpha::Discard,
            "force_vp8x" => options.force_vp8x = true,
            _ => panic!("unknown manifest options: {line}"),
        }
        let encoded = match fields[2] {
            "encode_rgb" => {
                let rgb: Vec<_> = fixture
                    .rgba
                    .chunks_exact(4)
                    .flat_map(|pixel| pixel[..3].iter().copied())
                    .collect();
                tiny_webp::encode_rgb(&rgb, fixture.width, fixture.height, &options)
            }
            "encode_rgba" => {
                tiny_webp::encode_rgba(&fixture.rgba, fixture.width, fixture.height, &options)
            }
            _ => panic!("unknown manifest entry point: {line}"),
        }
        .expect("encode the manifest row");
        assert_eq!(
            format!("{:x}", Sha256::digest(&encoded)),
            fields[4],
            "{line}"
        );
        let expected_alpha: Vec<_> = fixture.rgba.chunks_exact(4).map(|pixel| pixel[3]).collect();
        let has_alpha = fields[2] == "encode_rgba"
            && options.alpha == Alpha::Lossless
            && expected_alpha.iter().any(|alpha| *alpha != 255);
        let mut decoder = image_webp::WebPDecoder::new(Cursor::new(&encoded))
            .expect("decode the manifest header");
        assert_eq!(
            decoder.dimensions(),
            (fixture.width, fixture.height),
            "{line}"
        );
        assert_eq!(decoder.has_alpha(), has_alpha, "{line}");
        let channels = if has_alpha { 4 } else { 3 };
        let mut pixels = vec![0; fixture.width as usize * fixture.height as usize * channels];
        decoder
            .read_image(&mut pixels)
            .expect("decode the manifest pixels");
        if has_alpha {
            let alpha: Vec<_> = pixels.chunks_exact(4).map(|pixel| pixel[3]).collect();
            assert_eq!(alpha, expected_alpha, "image-webp {line}");
            alpha_rows += 1;
        }
        if oracle_available {
            std::fs::write(&input, &encoded).expect("write the manifest output");
            let result = Command::new("dwebp")
                .arg("-quiet")
                .arg(&input)
                .arg("-o")
                .arg(&output)
                .output()
                .expect("run dwebp");
            assert_eq!(
                result.status.code(),
                Some(0),
                "{line}: {}",
                String::from_utf8_lossy(&result.stderr)
            );
            let mut reader = png::Decoder::new(std::io::BufReader::new(
                std::fs::File::open(&output).expect("open the decoded PNG"),
            ))
            .read_info()
            .expect("read the decoded PNG header");
            let mut pixels = vec![0; reader.output_buffer_size().expect("size the decoded PNG")];
            let info = reader
                .next_frame(&mut pixels)
                .expect("decode the PNG pixels");
            assert_eq!(
                (info.width, info.height),
                (fixture.width, fixture.height),
                "{line}"
            );
            assert_eq!(info.bit_depth, png::BitDepth::Eight, "{line}");
            assert_eq!(
                info.color_type,
                if has_alpha {
                    png::ColorType::Rgba
                } else {
                    png::ColorType::Rgb
                },
                "{line}"
            );
            if has_alpha {
                let alpha: Vec<_> = pixels[..info.buffer_size()]
                    .chunks_exact(4)
                    .map(|pixel| pixel[3])
                    .collect();
                assert_eq!(alpha, expected_alpha, "dwebp {line}");
            }
        }
        rows += 1;
    }
    assert_eq!(rows, 2872);
    assert_eq!(alpha_rows, 306);
    std::fs::remove_dir_all(directory).expect("remove the test directory");
}
