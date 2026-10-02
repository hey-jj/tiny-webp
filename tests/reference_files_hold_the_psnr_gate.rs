#![forbid(unsafe_code)]

#[path = "../fixtures/generator.rs"]
mod generator;
#[path = "../fixtures/png_writer.rs"]
mod png_writer;

use sha2::{Digest, Sha256};
use std::fs;
use std::io::Cursor;
use std::path::PathBuf;
use std::process::Command;
use tiny_webp::{encode_rgba, Options};

struct Exemption {
    fixture: &'static str,
    quality: u8,
    recorded_gap_db: f64,
    closing_feature: &'static str,
}

const EXEMPTIONS: [Exemption; 2] = [
    Exemption {
        fixture: "flat",
        quality: 50,
        recorded_gap_db: 4.39,
        closing_feature: "four-segment quantization",
    },
    Exemption {
        fixture: "checker",
        quality: 75,
        recorded_gap_db: 5.01,
        closing_feature: "four-segment quantization",
    },
];

const QUALITIES: [u8; 5] = [25, 50, 75, 90, 95];
const MANIFEST_HEADER: &str =
    "libwebp 1.6.0\ncommand cwebp -quiet -q <q> <fixture>.png -o <fixture>-q<q>.webp\n";

fn reference_directory() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("fixtures/reference")
}

fn filename(name: &str, quality: u8) -> String {
    format!("{name}-q{quality}.webp")
}

#[test]
fn the_reference_manifest_pins_every_fixture_and_quality_in_bytewise_order() {
    let directory = reference_directory();
    let mut rows = Vec::new();
    let mut expected_files = vec!["manifest.txt".to_owned()];
    for fixture in generator::all() {
        for quality in QUALITIES {
            let name = filename(fixture.name, quality);
            let bytes = fs::read(directory.join(&name)).expect("read the reference image");
            rows.push(format!(
                "{} {quality} {:x}\n",
                fixture.name,
                Sha256::digest(&bytes)
            ));
            expected_files.push(name);
        }
    }
    rows.sort_unstable();
    let expected = format!("{MANIFEST_HEADER}{}", rows.concat());
    let actual = fs::read_to_string(directory.join("manifest.txt")).expect("read the manifest");
    assert_eq!(rows.len(), 75);
    assert_eq!(actual, expected);
    let mut actual_files: Vec<_> = fs::read_dir(directory)
        .expect("read the reference directory")
        .map(|entry| {
            entry
                .expect("read the reference entry")
                .file_name()
                .into_string()
                .expect("the reference filename is UTF-8")
        })
        .collect();
    actual_files.sort_unstable();
    expected_files.sort_unstable();
    assert_eq!(actual_files, expected_files);
}

struct Scratch(PathBuf);

impl Drop for Scratch {
    fn drop(&mut self) {
        fs::remove_dir_all(&self.0).expect("remove the test directory");
    }
}

#[test]
fn cwebp_regenerates_every_committed_reference_byte_for_byte() {
    let version = match Command::new("cwebp").arg("-version").output() {
        Ok(output) => output,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
            assert_ne!(
                std::env::var("TINY_WEBP_REQUIRE_ORACLE").as_deref(),
                Ok("1"),
                "cwebp is required"
            );
            eprintln!("cwebp is absent, skipping reference regeneration");
            return;
        }
        Err(error) => panic!("run cwebp: {error}"),
    };
    assert_eq!(version.status.code(), Some(0));
    assert_eq!(
        String::from_utf8_lossy(&version.stdout).lines().next(),
        Some("1.6.0")
    );
    let directory = std::env::temp_dir().join(format!(
        "tiny-webp-cwebp_regenerates_every_committed_reference_byte_for_byte-{}",
        std::process::id()
    ));
    fs::create_dir_all(&directory).expect("create the test directory");
    let scratch = Scratch(directory);
    for fixture in generator::all() {
        let input = scratch.0.join(format!("{}.png", fixture.name));
        png_writer::write(
            fs::File::create(&input).expect("create the fixture PNG"),
            fixture.width,
            fixture.height,
            png::ColorType::Rgba,
            &fixture.rgba,
        )
        .expect("write the fixture PNG");
        for quality in QUALITIES {
            let name = filename(fixture.name, quality);
            let output = scratch.0.join(&name);
            let process = Command::new("cwebp")
                .args(["-quiet", "-q", &quality.to_string()])
                .arg(&input)
                .arg("-o")
                .arg(&output)
                .output()
                .expect("run cwebp");
            assert_eq!(
                process.status.code(),
                Some(0),
                "{name}: {}",
                String::from_utf8_lossy(&process.stderr)
            );
            let generated = fs::read(output).expect("read the regenerated reference");
            let committed =
                fs::read(reference_directory().join(&name)).expect("read the committed reference");
            assert_eq!(
                generated,
                committed,
                "{name}: regenerated SHA-256 {:x}, committed SHA-256 {:x}",
                Sha256::digest(&generated),
                Sha256::digest(&committed)
            );
        }
    }
}

fn decoded_psnr(webp: &[u8], fixture: &generator::Fixture) -> f64 {
    let mut decoder =
        image_webp::WebPDecoder::new(Cursor::new(webp)).expect("decode the WebP header");
    assert_eq!(decoder.dimensions(), (fixture.width, fixture.height));
    let channels = if decoder.has_alpha() { 4 } else { 3 };
    let mut pixels = vec![0; fixture.width as usize * fixture.height as usize * channels];
    decoder
        .read_image(&mut pixels)
        .expect("decode the WebP pixels");
    rgb_psnr(&fixture.rgba, &pixels, channels)
}

fn rgb_psnr(source: &[u8], decoded: &[u8], channels: usize) -> f64 {
    assert_eq!(source.len() / 4, decoded.len() / channels);
    let mut squared_error = 0_u64;
    for (source, decoded) in source.chunks_exact(4).zip(decoded.chunks_exact(channels)) {
        for channel in 0..3 {
            let difference = i64::from(source[channel]) - i64::from(decoded[channel]);
            squared_error += (difference * difference) as u64;
        }
    }
    if squared_error == 0 {
        return 99.0;
    }
    let samples = source.len() / 4 * 3;
    let mean_squared_error = squared_error as f64 / samples as f64;
    10.0 * (255.0_f64.powi(2) / mean_squared_error).log10()
}

#[test]
fn rgb_psnr_excludes_alpha_and_counts_all_three_color_channels() {
    let source = [0, 0, 0, 0, 255, 255, 255, 255];
    assert_eq!(rgb_psnr(&source, &[0, 0, 0, 255, 255, 255], 3), 99.0);
    assert_eq!(
        rgb_psnr(&source, &[0, 0, 0, 255, 255, 255, 255, 0], 4),
        99.0
    );
    assert_eq!(rgb_psnr(&source, &[255, 255, 255, 0, 0, 0], 3), 0.0);
    assert_eq!(
        rgb_psnr(&source, &[255, 0, 0, 0, 255, 255], 3),
        10.0 * 3.0_f64.log10()
    );
}

#[test]
fn every_fixture_meets_the_psnr_limit_or_its_recorded_exemption() {
    let mut failures = Vec::new();
    let mut count = 0;
    for fixture in generator::all() {
        for quality in QUALITIES {
            let mut options = Options::default();
            options.quality = quality;
            let encoded = encode_rgba(&fixture.rgba, fixture.width, fixture.height, &options)
                .expect("encode the fixture");
            let reference = fs::read(reference_directory().join(filename(fixture.name, quality)))
                .expect("read the reference image");
            let actual = decoded_psnr(&encoded, &fixture);
            let expected = decoded_psnr(&reference, &fixture);
            let gap = expected - actual;
            let exemption = EXEMPTIONS
                .iter()
                .find(|row| row.fixture == fixture.name && row.quality == quality);
            let exempt = exemption.is_some_and(|row| gap <= row.recorded_gap_db + 0.1);
            if gap > 3.0 && !exempt {
                failures.push(format!(
                    "{} q{quality}: tiny-webp {actual:.9} dB, cwebp {expected:.9} dB, feature {}",
                    fixture.name,
                    exemption.map_or("none", |row| row.closing_feature)
                ));
            }
            count += 1;
        }
    }
    assert_eq!(count, 75);
    assert_eq!(failures, Vec::<String>::new());
}
