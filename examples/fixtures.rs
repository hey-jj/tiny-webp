//! Prints the fixture table or writes it as PNG files.

#[path = "../fixtures/generator.rs"]
mod generator;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    if let Some(directory) = std::env::args_os().nth(1) {
        generator::write_all(std::path::Path::new(&directory))?;
    } else {
        print_table();
    }
    Ok(())
}

/// Prints one table row for every fixture.
fn print_table() {
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
