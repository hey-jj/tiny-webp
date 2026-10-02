//! Measures encoder speed, peak heap growth, output size, and RGB PSNR.

// The std::alloc::GlobalAlloc contract requires unsafe allocator methods.
// This counting allocator is the crate root exception.
#![deny(unsafe_op_in_unsafe_fn)]

#[path = "../fixtures/generator.rs"]
mod generator;
#[path = "../fixtures/png_writer.rs"]
mod png_writer;

use std::alloc::{GlobalAlloc, Layout, System};
use std::error::Error;
use std::fs;
use std::io::Cursor;
use std::path::{Path, PathBuf};
use std::process::{Command, ExitStatus};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::time::{Duration, Instant};

use tiny_webp::Options;

struct CountingAllocator;

static LIVE_HEAP_BYTES: AtomicUsize = AtomicUsize::new(0);
static PEAK_HEAP_BYTES: AtomicUsize = AtomicUsize::new(0);

#[global_allocator]
static ALLOCATOR: CountingAllocator = CountingAllocator;

// The std::alloc::GlobalAlloc contract forbids unwinding in allocator methods.
// These methods forward valid arguments to System and count bytes with atomics.
unsafe impl GlobalAlloc for CountingAllocator {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        // std::alloc::GlobalAlloc requires a nonzero layout with valid alignment.
        let pointer = unsafe { System.alloc(layout) };
        if !pointer.is_null() {
            record_growth(layout.size());
        }
        pointer
    }

    unsafe fn alloc_zeroed(&self, layout: Layout) -> *mut u8 {
        // std::alloc::GlobalAlloc requires a nonzero layout with valid alignment.
        let pointer = unsafe { System.alloc_zeroed(layout) };
        if !pointer.is_null() {
            record_growth(layout.size());
        }
        pointer
    }

    unsafe fn dealloc(&self, pointer: *mut u8, layout: Layout) {
        // std::alloc::GlobalAlloc requires this pointer and layout to match an allocation.
        // Every allocation here comes from System.
        unsafe { System.dealloc(pointer, layout) };
        record_shrink(layout.size());
    }

    unsafe fn realloc(&self, pointer: *mut u8, layout: Layout, new_size: usize) -> *mut u8 {
        // std::alloc::GlobalAlloc requires a live allocation and a valid new size.
        // System owns each pointer and receives its unchanged layout.
        let new_pointer = unsafe { System.realloc(pointer, layout, new_size) };
        if !new_pointer.is_null() {
            if new_size >= layout.size() {
                record_growth(new_size - layout.size());
            } else {
                record_shrink(layout.size() - new_size);
            }
        }
        new_pointer
    }
}

fn record_growth(bytes: usize) {
    let live = LIVE_HEAP_BYTES.fetch_add(bytes, Ordering::Relaxed) + bytes;
    PEAK_HEAP_BYTES.fetch_max(live, Ordering::Relaxed);
}

fn record_shrink(bytes: usize) {
    LIVE_HEAP_BYTES.fetch_sub(bytes, Ordering::Relaxed);
}

fn begin_heap_measurement() -> usize {
    let baseline = LIVE_HEAP_BYTES.load(Ordering::Relaxed);
    PEAK_HEAP_BYTES.store(baseline, Ordering::Relaxed);
    baseline
}

fn peak_heap_growth(baseline: usize) -> usize {
    PEAK_HEAP_BYTES
        .load(Ordering::Relaxed)
        .saturating_sub(baseline)
}

fn main() -> Result<(), Box<dyn Error>> {
    let smoke = parse_arguments()?;
    println!(
        "tiny-webp {} on {} {}",
        env!("CARGO_PKG_VERSION"),
        std::env::consts::OS,
        std::env::consts::ARCH
    );

    let cwebp = !smoke && cwebp_is_available();
    let scratch = cwebp.then(prepare_scratch).transpose()?;
    let result = run(smoke, scratch.as_deref());
    if let Some(directory) = scratch {
        fs::remove_dir_all(directory)?;
    }
    result
}

fn parse_arguments() -> Result<bool, Box<dyn Error>> {
    let arguments: Vec<_> = std::env::args_os().skip(1).collect();
    match arguments.as_slice() {
        [] => Ok(false),
        [flag] if flag == "--smoke" => Ok(true),
        _ => Err("usage: bench [--smoke]".into()),
    }
}

fn cwebp_is_available() -> bool {
    Command::new("cwebp")
        .arg("-version")
        .output()
        .map(|output| output.status.success())
        .unwrap_or(false)
}

fn prepare_scratch() -> Result<PathBuf, Box<dyn Error>> {
    let directory = std::env::temp_dir().join(format!("tiny-webp-bench-{}", std::process::id()));
    if directory.exists() {
        fs::remove_dir_all(&directory)?;
    }
    fs::create_dir_all(&directory)?;
    for fixture in generator::all() {
        let path = directory.join(format!("{}.png", fixture.name));
        let output = std::io::BufWriter::new(fs::File::create(path)?);
        png_writer::write(
            output,
            fixture.width,
            fixture.height,
            png::ColorType::Rgba,
            &fixture.rgba,
        )?;
    }
    Ok(directory)
}

fn run(smoke: bool, scratch: Option<&Path>) -> Result<(), Box<dyn Error>> {
    let binary = std::env::current_exe()?
        .parent()
        .and_then(Path::parent)
        .ok_or("the example needs its Cargo build directory")?
        .join(format!("tiny-webp{}", std::env::consts::EXE_SUFFIX));
    let fixtures = generator::all();
    let mut memory_failures = Vec::new();
    let qualities: &[u8] = if smoke { &[75] } else { &[50, 75, 90] };
    for quality in qualities {
        let mut options = Options::default();
        options.quality = *quality;
        for fixture in &fixtures {
            if smoke && !smoke_fixture(fixture.name) {
                continue;
            }
            let pixels = fixture.width as usize * fixture.height as usize;
            let baseline = begin_heap_measurement();
            let started = Instant::now();
            let encoded =
                tiny_webp::encode_rgba(&fixture.rgba, fixture.width, fixture.height, &options)?;
            let elapsed = started.elapsed();
            let peak_bytes = peak_heap_growth(baseline);
            let memory_bound_held = memory_bound_holds(pixels, peak_bytes);
            let psnr = rgb_psnr(&encoded, &fixture.rgba, fixture.width, fixture.height)?;
            print!(
                "{} q{} megapixels_per_second={:.3} peak_heap_bytes_per_pixel={:.3} bytes={} rgb_psnr_db={:.3} peak_heap_bytes={} memory_bound_held={}",
                fixture.name,
                quality,
                megapixels_per_second(pixels, elapsed),
                peak_bytes as f64 / pixels as f64,
                encoded.len(),
                psnr,
                peak_bytes,
                if memory_bound_held { "yes" } else { "no" }
            );

            if let Some(directory) = scratch {
                let comparison = compare_processes(&binary, directory, fixture, *quality)?;
                let comparison_psnr = rgb_psnr(
                    &comparison.bytes,
                    &fixture.rgba,
                    fixture.width,
                    fixture.height,
                )?;
                print!(
                    " cwebp_bytes={} tiny_webp_to_cwebp_size_ratio={:.3} cwebp_rgb_psnr_db={:.3} tiny_webp_subprocess_ms={:.3} cwebp_subprocess_ms={:.3} tiny_webp_to_cwebp_time_ratio={:.3}",
                    comparison.bytes.len(),
                    encoded.len() as f64 / comparison.bytes.len() as f64,
                    comparison_psnr,
                    comparison.tiny_webp_elapsed.as_secs_f64() * 1000.0,
                    comparison.cwebp_elapsed.as_secs_f64() * 1000.0,
                    comparison.tiny_webp_elapsed.as_secs_f64()
                        / comparison.cwebp_elapsed.as_secs_f64()
                );
            }
            println!();
            if !memory_bound_held {
                memory_failures.push(format!(
                    "{} at q{} exceeds the memory bound with peak heap growth {} and output size {}",
                    fixture.name, quality, peak_bytes, encoded.len()
                ));
            }
        }
    }
    if memory_failures.is_empty() {
        Ok(())
    } else {
        Err(memory_failures.join("\n").into())
    }
}

fn megapixels_per_second(pixels: usize, elapsed: Duration) -> f64 {
    pixels as f64 / 1_000_000.0 / elapsed.as_secs_f64()
}

fn smoke_fixture(name: &str) -> bool {
    matches!(name, "flat" | "photo-large")
}

fn memory_bound_holds(pixels: usize, peak_bytes: usize) -> bool {
    peak_bytes + 4 * pixels <= 8 * pixels + 1024 * 1024
}

struct Comparison {
    bytes: Vec<u8>,
    tiny_webp_elapsed: Duration,
    cwebp_elapsed: Duration,
}

fn compare_processes(
    binary: &Path,
    directory: &Path,
    fixture: &generator::Fixture,
    quality: u8,
) -> Result<Comparison, Box<dyn Error>> {
    let input = directory.join(format!("{}.png", fixture.name));
    let tiny_output = directory.join(format!("{}-q{}-tiny.webp", fixture.name, quality));
    let cwebp_output = directory.join(format!("{}-q{}-cwebp.webp", fixture.name, quality));
    let mut tiny_command = encoder_command(binary, &input, &tiny_output, quality);
    let mut cwebp_command = encoder_command(Path::new("cwebp"), &input, &cwebp_output, quality);
    measure_process(&mut tiny_command)?;
    measure_process(&mut cwebp_command)?;
    let mut tiny_times = [Duration::ZERO; 5];
    let mut cwebp_times = [Duration::ZERO; 5];
    for index in 0..5 {
        tiny_times[index] = measure_process(&mut tiny_command)?;
        cwebp_times[index] = measure_process(&mut cwebp_command)?;
    }
    Ok(Comparison {
        bytes: fs::read(cwebp_output)?,
        tiny_webp_elapsed: median(tiny_times),
        cwebp_elapsed: median(cwebp_times),
    })
}

fn encoder_command(binary: &Path, input: &Path, output: &Path, quality: u8) -> Command {
    let mut command = Command::new(binary);
    command
        .arg("-quiet")
        .arg("-q")
        .arg(quality.to_string())
        .arg(input)
        .arg("-o")
        .arg(output);
    command
}

fn measure_process(command: &mut Command) -> Result<Duration, Box<dyn Error>> {
    let started = Instant::now();
    let status = command.status()?;
    let elapsed = started.elapsed();
    require_success(status, command)?;
    Ok(elapsed)
}

fn median(mut times: [Duration; 5]) -> Duration {
    times.sort_unstable();
    times[2]
}

fn require_success(status: ExitStatus, command: &Command) -> Result<(), Box<dyn Error>> {
    if status.success() {
        Ok(())
    } else {
        Err(format!("{command:?} failed with {status}").into())
    }
}

fn rgb_psnr(webp: &[u8], source: &[u8], width: u32, height: u32) -> Result<f64, Box<dyn Error>> {
    let mut decoder = image_webp::WebPDecoder::new(Cursor::new(webp))?;
    if decoder.dimensions() != (width, height) {
        return Err(format!(
            "decoded dimensions {:?} differ from {width}x{height}",
            decoder.dimensions()
        )
        .into());
    }
    let channels = if decoder.has_alpha() { 4 } else { 3 };
    let mut decoded = vec![0; width as usize * height as usize * channels];
    decoder.read_image(&mut decoded)?;

    let squared_error: u64 = source
        .chunks_exact(4)
        .zip(decoded.chunks_exact(channels))
        .map(|(source_pixel, decoded_pixel)| {
            source_pixel[..3]
                .iter()
                .zip(&decoded_pixel[..3])
                .map(|(source_channel, decoded_channel)| {
                    let difference = i32::from(*source_channel) - i32::from(*decoded_channel);
                    (difference * difference) as u64
                })
                .sum::<u64>()
        })
        .sum();
    if squared_error == 0 {
        return Ok(99.0);
    }
    let sample_count = u64::from(width) * u64::from(height) * 3;
    Ok(10.0 * ((255.0 * 255.0 * sample_count as f64) / squared_error as f64).log10())
}

#[cfg(test)]
mod tests {
    use super::{median, memory_bound_holds, smoke_fixture};
    use std::time::Duration;

    #[test]
    fn the_memory_bound_counts_the_callers_rgba_bytes() {
        let pixels = 1024 * 768;
        let limit = 4 * pixels + 1024 * 1024;
        assert_eq!(
            (
                memory_bound_holds(pixels, limit),
                memory_bound_holds(pixels, limit + 1)
            ),
            (true, false)
        );
    }

    #[test]
    fn the_median_selects_the_third_of_five_ordered_durations() {
        assert_eq!(
            median([8, 1, 9, 3, 2].map(Duration::from_millis)),
            Duration::from_millis(3)
        );
    }

    #[test]
    fn smoke_mode_includes_flat_and_photo_large() {
        let names: Vec<_> = super::generator::all()
            .into_iter()
            .filter(|fixture| smoke_fixture(fixture.name))
            .map(|fixture| fixture.name)
            .collect();
        assert_eq!(names, ["flat", "photo-large"]);
    }
}
