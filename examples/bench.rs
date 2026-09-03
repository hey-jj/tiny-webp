//! Runs the fixture set through the encoder at three qualities.

#[path = "../fixtures/generator.rs"]
mod generator;

use tiny_webp::Options;

fn main() {
    println!(
        "tiny-webp {} on {} {}",
        env!("CARGO_PKG_VERSION"),
        std::env::consts::OS,
        std::env::consts::ARCH
    );

    let fixtures = generator::all();
    for quality in [50u8, 75, 90] {
        let mut opts = Options::default();
        opts.quality = quality;
        for fixture in &fixtures {
            let outcome =
                match tiny_webp::encode_rgba(&fixture.rgba, fixture.width, fixture.height, &opts) {
                    Ok(webp) => format!("{} bytes", webp.len()),
                    Err(err) => err.to_string(),
                };
            println!("{} q{} {}", fixture.name, quality, outcome);
        }
    }
}
