//! Prints the fixture table.

#[path = "../fixtures/generator.rs"]
mod generator;

fn main() {
    for fixture in generator::all() {
        let transparent = fixture
            .rgba
            .iter()
            .skip(3)
            .step_by(4)
            .any(|byte| *byte < 255);
        println!(
            "{} {}x{} alpha={} bytes={}",
            fixture.name,
            fixture.width,
            fixture.height,
            transparent,
            fixture.rgba.len()
        );
    }
}
