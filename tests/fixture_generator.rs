//! The generator gives the same bytes on every run.

#[path = "../fixtures/generator.rs"]
mod generator;
#[path = "../fixtures/png_writer.rs"]
mod png_writer;

use std::fs::File;
use std::io::{BufReader, BufWriter};
use std::path::Path;

#[test]
fn two_runs_of_the_generator_agree_byte_for_byte_on_every_fixture() {
    let first = generator::all();
    let second = generator::all();
    assert_eq!(first.len(), second.len());
    for (left, right) in first.iter().zip(second.iter()) {
        assert_eq!(
            (left.name, left.width, left.height),
            (right.name, right.width, right.height)
        );
        assert_eq!(left.rgba, right.rgba, "{} moved between runs", left.name);
    }
}

#[test]
fn every_fixture_holds_four_bytes_for_each_of_its_pixels() {
    for fixture in generator::all() {
        assert_eq!(
            fixture.rgba.len(),
            fixture.width as usize * fixture.height as usize * 4,
            "{}",
            fixture.name
        );
    }
}

#[test]
fn the_fixture_table_contains_the_four_milestone_one_images() {
    let fixtures = generator::all();
    let table: Vec<(&str, u32, u32)> = fixtures
        .iter()
        .map(|fixture| (fixture.name, fixture.width, fixture.height))
        .collect();
    assert_eq!(
        table,
        vec![
            ("flat", 32, 32),
            ("checker", 32, 32),
            ("gradient", 64, 48),
            ("text-blocks", 64, 48),
            ("noise", 64, 48),
            ("lowpass-noise", 64, 48),
            ("alpha-soft", 64, 48),
            ("alpha-hard", 64, 48),
            ("alpha-odd", 17, 31),
            ("photo-large", 1024, 768),
            ("one-pixel", 1, 1),
            ("single-column", 1, 33),
            ("single-row", 33, 1),
            ("odd-size", 17, 31),
        ]
    );
}

#[test]
fn the_flat_and_checker_fixtures_hold_their_exact_colors() {
    let fixtures = generator::all();
    let flat = fixtures
        .iter()
        .find(|fixture| fixture.name == "flat")
        .expect("the table carries flat");
    assert_eq!(flat.rgba, [96, 128, 160, 255].repeat(32 * 32));

    let checker = fixtures
        .iter()
        .find(|fixture| fixture.name == "checker")
        .expect("the table carries checker");
    for y in 0..32usize {
        for x in 0..32usize {
            let value = if (x / 2 + y / 2) % 2 == 0 { 0 } else { 255 };
            let index = (y * 32 + x) * 4;
            assert_eq!(&checker.rgba[index..index + 4], &[value, value, value, 255]);
        }
    }
}

#[test]
fn every_written_png_decodes_to_the_generated_rgba_bytes() {
    let directory =
        std::env::temp_dir().join(format!("tiny-webp-unit-8-fixtures-{}", std::process::id()));
    if directory.exists() {
        std::fs::remove_dir_all(&directory).expect("remove the old test directory");
    }
    write_all(&directory);
    let written = std::fs::read_dir(&directory)
        .expect("read the fixture directory")
        .count();
    assert_eq!(written, 14);

    for fixture in generator::all() {
        let input = File::open(directory.join(format!("{}.png", fixture.name)))
            .expect("open the fixture file");
        let decoder = png::Decoder::new(BufReader::new(input));
        let mut reader = decoder.read_info().expect("read the PNG header");
        let mut decoded = vec![0; reader.output_buffer_size().expect("get the PNG size")];
        let info = reader
            .next_frame(&mut decoded)
            .expect("read the PNG pixels");
        decoded.truncate(info.buffer_size());

        assert_eq!((info.width, info.height), (fixture.width, fixture.height));
        assert_eq!(info.color_type, png::ColorType::Rgba);
        assert_eq!(info.bit_depth, png::BitDepth::Eight);
        assert_eq!(decoded, fixture.rgba, "{}", fixture.name);
    }

    std::fs::remove_dir_all(directory).expect("remove the test directory");
}

/// Writes the generated pixels as RGBA8 PNG files for this test.
fn write_all(directory: &Path) {
    std::fs::create_dir_all(directory).expect("create the fixture directory");
    for fixture in generator::all() {
        let path = directory.join(format!("{}.png", fixture.name));
        let output = BufWriter::new(File::create(path).expect("create the fixture file"));
        png_writer::write(
            output,
            fixture.width,
            fixture.height,
            png::ColorType::Rgba,
            &fixture.rgba,
        )
        .expect("write the PNG pixels");
    }
}
