//! Prints the fixture table or writes it as PNG files.

#[path = "../fixtures/generator.rs"]
mod generator;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    if let Some(directory) = std::env::args_os().nth(1) {
        write_all(std::path::Path::new(&directory))?;
    } else {
        print_table();
    }
    Ok(())
}

/// Writes every fixture as an RGBA8 PNG file.
fn write_all(directory: &std::path::Path) -> Result<(), Box<dyn std::error::Error>> {
    std::fs::create_dir_all(directory)?;
    for fixture in generator::all() {
        let path = directory.join(format!("{}.png", fixture.name));
        let output = std::io::BufWriter::new(std::fs::File::create(path)?);
        let mut encoder = png::Encoder::new(output, fixture.width, fixture.height);
        encoder.set_color(png::ColorType::Rgba);
        encoder.set_depth(png::BitDepth::Eight);
        let mut writer = encoder.write_header()?;
        writer.write_image_data(&fixture.rgba)?;
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
