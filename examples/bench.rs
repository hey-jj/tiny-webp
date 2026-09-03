//! Measures encoder speed, peak heap growth, output size, and RGB PSNR.

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

unsafe impl GlobalAlloc for CountingAllocator {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        // GlobalAlloc requires callers to supply a valid layout.
        let pointer = unsafe { System.alloc(layout) };
        if !pointer.is_null() {
            record_growth(layout.size());
        }
        pointer
    }

    unsafe fn alloc_zeroed(&self, layout: Layout) -> *mut u8 {
        // GlobalAlloc requires callers to supply a valid layout.
        let pointer = unsafe { System.alloc_zeroed(layout) };
        if !pointer.is_null() {
            record_growth(layout.size());
        }
        pointer
    }

    unsafe fn dealloc(&self, pointer: *mut u8, layout: Layout) {
        // GlobalAlloc returns this allocator's pointers to dealloc.
        unsafe { System.dealloc(pointer, layout) };
        record_shrink(layout.size());
    }

    unsafe fn realloc(&self, pointer: *mut u8, layout: Layout, new_size: usize) -> *mut u8 {
        // GlobalAlloc returns this allocator's pointers to realloc.
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
    let fixtures = generator::all();
    let qualities: &[u8] = if smoke { &[75] } else { &[50, 75, 90] };
    for quality in qualities {
        let mut options = Options::default();
        options.quality = *quality;
        for fixture in &fixtures {
            if smoke && fixture.name != "flat" {
                continue;
            }
            let pixels = fixture.width as usize * fixture.height as usize;
            let baseline = begin_heap_measurement();
            let started = Instant::now();
            let encoded =
                tiny_webp::encode_rgba(&fixture.rgba, fixture.width, fixture.height, &options)?;
            let elapsed = started.elapsed();
            let peak_bytes = peak_heap_growth(baseline);
            let psnr = rgb_psnr(&encoded, &fixture.rgba, fixture.width, fixture.height)?;
            print!(
                "{} q{} megapixels_per_second={:.3} peak_heap_bytes_per_pixel={:.3} bytes={} rgb_psnr_db={:.3}",
                fixture.name,
                quality,
                megapixels_per_second(pixels, elapsed),
                peak_bytes as f64 / pixels as f64,
                encoded.len(),
                psnr
            );

            if let Some(directory) = scratch {
                let comparison = run_cwebp(directory, fixture, *quality)?;
                let comparison_psnr = rgb_psnr(
                    &comparison.bytes,
                    &fixture.rgba,
                    fixture.width,
                    fixture.height,
                )?;
                print!(
                    " cwebp_bytes={} tiny_webp_to_cwebp_size_ratio={:.3} cwebp_rgb_psnr_db={:.3} cwebp_subprocess_ms={:.3}",
                    comparison.bytes.len(),
                    encoded.len() as f64 / comparison.bytes.len() as f64,
                    comparison_psnr,
                    comparison.elapsed.as_secs_f64() * 1000.0
                );
            }
            println!();
        }
    }
    Ok(())
}

fn megapixels_per_second(pixels: usize, elapsed: Duration) -> f64 {
    pixels as f64 / 1_000_000.0 / elapsed.as_secs_f64()
}

struct Comparison {
    bytes: Vec<u8>,
    elapsed: Duration,
}

fn run_cwebp(
    directory: &Path,
    fixture: &generator::Fixture,
    quality: u8,
) -> Result<Comparison, Box<dyn Error>> {
    let input = directory.join(format!("{}.png", fixture.name));
    let output = directory.join(format!("{}-q{}.webp", fixture.name, quality));
    let started = Instant::now();
    let status = Command::new("cwebp")
        .arg("-quiet")
        .arg("-q")
        .arg(quality.to_string())
        .arg(&input)
        .arg("-o")
        .arg(&output)
        .status()?;
    let elapsed = started.elapsed();
    require_success(status, fixture.name, quality)?;
    Ok(Comparison {
        bytes: fs::read(output)?,
        elapsed,
    })
}

fn require_success(status: ExitStatus, fixture: &str, quality: u8) -> Result<(), Box<dyn Error>> {
    if status.success() {
        Ok(())
    } else {
        Err(format!("cwebp failed for {fixture} at q{quality} with {status}").into())
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
        return Ok(f64::INFINITY);
    }
    let sample_count = u64::from(width) * u64::from(height) * 3;
    Ok(10.0 * ((255.0 * 255.0 * sample_count as f64) / squared_error as f64).log10())
}
