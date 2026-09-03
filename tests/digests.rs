//! The committed digest manifest matches every fixture encode.

#[path = "../examples/digests.rs"]
mod digests;

use std::sync::OnceLock;

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
    expected.sort_unstable();

    assert_eq!(actual.len(), 2668);
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
