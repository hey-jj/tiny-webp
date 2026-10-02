//! The `tiny-webp` command line.

#![forbid(unsafe_code)]

use std::collections::VecDeque;
use std::ffi::{OsStr, OsString};
use std::io::{Cursor, Read, Write};
use std::path::Path;
use std::process::ExitCode;
use std::time::Instant;

use lexopt::prelude::{Long, Short, Value};
use tiny_webp::{Alpha, Filter, Options};

const USAGE: &str = "\
usage: tiny-webp [options] <input> -o <output.webp>

  -q <0..100>, --quality <0..100>   quality, default 75
  -o <file>,   --output <file>      output path, or - for stdout
  -noalpha                          drop the alpha plane
  -f <0..100>                       loop filter strength, 0 turns it off
  -sharpness <0..7>                 loop filter sharpness, needs -f
  -print_psnr                       decode the result and print PSNR against the input
  -quiet                            no output on success
  -v                                print dimensions, bytes, and encode time
  -version, --version
  -h, --help

Filter strength S sets the level to round(S * 63 / 100).
";
const PROGRAM_PREFIX: &str = "tiny-webp: ";
const VERSION_PREFIX: &str = "tiny-webp ";
const STDOUT_NAME: &str = "stdout";
const UNKNOWN_FLAG: &str = "Unknown flag {flag}. Expected a supported option.";
const UNKNOWN_OPTION: &str = "Unknown command line option {option}. Expected a supported option.";
const INVALID_QUALITY: &str = "Invalid quality {quality}. Expected a whole number from 0 to 100.";
const MISSING_QUALITY: &str =
    "Missing value for -q. Pass a quality integer between zero and one hundred.";
const INVALID_STRENGTH: &str =
    "Invalid filter strength {value}. Expected a whole number from 0 to 100.";
const INVALID_SHARPNESS: &str = "Invalid sharpness {value}. Expected a whole number from 0 to 7.";
const MISSING_STRENGTH: &str = "Missing value for -f. Pass a whole number from 0 to 100.";
const MISSING_SHARPNESS: &str = "Missing value for -sharpness. Pass a whole number from 0 to 7.";
const SHARPNESS_NEEDS_STRENGTH: &str = "Pass -f with -sharpness to set the loop filter level.";
const MISSING_OUTPUT_VALUE: &str = "Missing value for -o. Expected an output path or -.";
const MISSING_INPUT: &str = "Missing input path. Pass a file or - for stdin.";
const MISSING_OUTPUT: &str = "Missing output path. Pass -o <file> or -o - for stdout.";
const SECOND_INPUT: &str = "Unexpected input path {name}. tiny-webp reads one input path.";
const READ_PATH: &str = "Could not read {name}. Check that the input path is readable.";
const READ_STDIN: &str = "Could not read stdin. Check that standard input is readable.";
const UNSUPPORTED: &str = "Could not decode {name}. Expected PNG, JPEG, or WebP bytes.";
const PNG_ERROR: &str = "Could not decode {name} as PNG.";
const JPEG_ERROR: &str = "Could not decode {name} as JPEG.";
const CMYK_ERROR: &str = "Could not decode {name}. CMYK JPEG input is unsupported.";
const WEBP_ERROR: &str = "Could not decode {name} as WebP.";
const ANIMATED_ERROR: &str = "Could not decode {name}. Animated WebP input is unsupported.";
const ENCODE_ERROR: &str =
    "Could not encode {name}. Expected dimensions from 1 through 16383 per side.";
const WRITE_PATH: &str = "Could not write {name}. Check that the output path is writable.";
const WRITE_STDOUT: &str = "Could not write stdout. Check that standard output is writable.";
const PSNR_LINE: &str = "tiny-webp: psnr {value} dB";
const SUMMARY: &str = "tiny-webp: wrote {bytes} bytes to {output}";
const VERBOSE_SUMMARY: &str = "tiny-webp: {width}x{height}, {bytes} bytes, {ms}.{micros} ms";

enum Action {
    Help,
    Version,
    Encode(Cli),
}

struct Cli {
    input: OsString,
    output: OsString,
    options: Options,
    quiet: bool,
    verbose: bool,
    print_psnr: bool,
}

enum Pixels {
    Rgb(Vec<u8>),
    Rgba(Vec<u8>),
}

struct Image {
    pixels: Pixels,
    width: u32,
    height: u32,
}

struct Success {
    width: u32,
    height: u32,
    byte_count: usize,
    micros: u128,
    output: OsString,
    quiet: bool,
    verbose: bool,
    psnr: Option<f64>,
}

fn main() -> ExitCode {
    match parse(std::env::args_os().skip(1)) {
        Ok(Action::Help) => print_stdout(USAGE.as_bytes()),
        Ok(Action::Version) => {
            print_stdout(format!("{VERSION_PREFIX}{}\n", env!("CARGO_PKG_VERSION")).as_bytes())
        }
        Ok(Action::Encode(cli)) => match encode(cli) {
            Ok(success) => {
                if !success.quiet {
                    if let Some(value) = success.psnr {
                        eprintln!("{}", PSNR_LINE.replace("{value}", &format!("{value:.2}")));
                    }
                    eprintln!("{}", summary(&success));
                }
                ExitCode::SUCCESS
            }
            Err(problem) => {
                eprintln!("{PROGRAM_PREFIX}{problem}");
                ExitCode::from(1)
            }
        },
        Err(problem) => {
            eprintln!("{PROGRAM_PREFIX}{problem}");
            eprint!("{USAGE}");
            ExitCode::from(2)
        }
    }
}

fn print_stdout(bytes: &[u8]) -> ExitCode {
    match write_output(OsStr::new("-"), bytes) {
        Ok(()) => ExitCode::SUCCESS,
        Err(problem) => {
            eprintln!("{PROGRAM_PREFIX}{problem}");
            ExitCode::from(1)
        }
    }
}

fn parse<I>(args: I) -> Result<Action, String>
where
    I: IntoIterator<Item = OsString>,
{
    let mut input: Option<OsString> = None;
    let mut output: Option<OsString> = None;
    let mut options = Options::default();
    let mut quiet = false;
    let mut verbose = false;
    let mut print_psnr = false;
    let mut strength = None;
    let mut sharpness = None;
    let mut flag_names = VecDeque::new();

    let mut values_only = false;
    let mut value_next = false;
    let args: Vec<_> = args
        .into_iter()
        .map(|arg| {
            if values_only || value_next {
                value_next = false;
                return arg;
            }
            let bytes = arg.as_os_str().as_encoded_bytes();
            if bytes.starts_with(b"-") && bytes != b"-" && bytes != b"--" {
                flag_names.push_back(arg.clone());
            }
            let arg = expand_single_dash(arg);
            let bytes = arg.as_os_str().as_encoded_bytes();
            values_only = bytes == b"--";
            value_next = matches!(
                bytes,
                b"-q" | b"--quality" | b"-o" | b"--output" | b"-f" | b"--sharpness"
            );
            arg
        })
        .collect();
    let mut parser = lexopt::Parser::from_args(args);
    while let Some(arg) = parser.next().map_err(|_| {
        UNKNOWN_OPTION.replace(
            "{option}",
            &display_path(
                flag_names
                    .front()
                    .map_or(OsStr::new(""), OsString::as_os_str),
            ),
        )
    })? {
        let flag_name = if matches!(arg, Short(_) | Long(_)) {
            flag_names.pop_front().unwrap_or_default()
        } else {
            OsString::new()
        };
        match arg {
            Short('h') | Long("help") => {
                require_word_flag(&mut parser, &flag_name)?;
                return Ok(Action::Help);
            }
            Long("version") => {
                require_word_flag(&mut parser, &flag_name)?;
                return Ok(Action::Version);
            }
            Short('q') | Long("quality") => {
                let raw = parser.value().map_err(|_| MISSING_QUALITY.to_owned())?;
                options.quality = parse_quality(&raw)?;
            }
            Short('o') | Long("output") => {
                output = Some(
                    parser
                        .value()
                        .map_err(|_| MISSING_OUTPUT_VALUE.to_owned())?,
                );
            }
            Short('f') => {
                let raw = parser.value().map_err(|_| MISSING_STRENGTH.to_owned())?;
                strength = Some(parse_bounded(&raw, 100, INVALID_STRENGTH)?);
            }
            Long("sharpness") => {
                let raw = parser.value().map_err(|_| MISSING_SHARPNESS.to_owned())?;
                sharpness = Some(parse_bounded(&raw, 7, INVALID_SHARPNESS)?);
            }
            Long("print_psnr") => {
                require_word_flag(&mut parser, &flag_name)?;
                print_psnr = true;
            }
            Long("noalpha") => {
                require_word_flag(&mut parser, &flag_name)?;
                options.alpha = Alpha::Discard;
            }
            Long("quiet") => {
                require_word_flag(&mut parser, &flag_name)?;
                quiet = true;
            }
            Short('v') => {
                require_word_flag(&mut parser, &flag_name)?;
                verbose = true;
            }
            Long(_) | Short(_) => {
                return Err(UNKNOWN_FLAG.replace("{flag}", &display_path(&flag_name)));
            }
            Value(path) => {
                if input.replace(path.clone()).is_some() {
                    return Err(named_problem(SECOND_INPUT, &path));
                }
            }
        }
    }

    if let Some(strength) = strength {
        options.filter = Filter::Level {
            level: ((u16::from(strength) * 63 + 50) / 100) as u8,
            sharpness: sharpness.unwrap_or(0),
        };
    } else if sharpness.is_some() {
        return Err(SHARPNESS_NEEDS_STRENGTH.to_owned());
    }
    let input = input.ok_or_else(|| MISSING_INPUT.to_owned())?;
    let output = output.ok_or_else(|| MISSING_OUTPUT.to_owned())?;
    Ok(Action::Encode(Cli {
        input,
        output,
        options,
        quiet,
        verbose,
        print_psnr,
    }))
}

fn require_word_flag(parser: &mut lexopt::Parser, name: &OsStr) -> Result<(), String> {
    if parser.optional_value().is_some() {
        Err(UNKNOWN_OPTION.replace("{option}", &display_path(name)))
    } else {
        Ok(())
    }
}

fn expand_single_dash(arg: OsString) -> OsString {
    let bytes = arg.as_os_str().as_encoded_bytes();
    if bytes.len() > 2 && bytes[0] == b'-' && bytes[1] != b'-' {
        let mut expanded = OsString::from("-");
        expanded.push(arg);
        expanded
    } else {
        arg
    }
}

fn parse_quality(raw: &OsStr) -> Result<u8, String> {
    let text = raw.to_string_lossy();
    match text.parse::<u8>() {
        Ok(quality) if quality <= 100 => Ok(quality),
        _ => Err(INVALID_QUALITY.replace("{quality}", &text)),
    }
}

fn parse_bounded(raw: &OsStr, maximum: u8, problem: &str) -> Result<u8, String> {
    let text = raw.to_string_lossy();
    match text.parse::<u8>() {
        Ok(value) if value <= maximum => Ok(value),
        _ => Err(problem.replace("{value}", &display_path(raw))),
    }
}

fn encode(cli: Cli) -> Result<Success, String> {
    let bytes = read_input(&cli.input)?;
    let image = decode_input(&bytes, &cli.input)?;
    let started = Instant::now();
    let encoded = match &image.pixels {
        Pixels::Rgb(pixels) => {
            tiny_webp::encode_rgb(pixels, image.width, image.height, &cli.options)
        }
        Pixels::Rgba(pixels) => {
            tiny_webp::encode_rgba(pixels, image.width, image.height, &cli.options)
        }
    }
    .map_err(|_| named_problem(ENCODE_ERROR, &cli.input))?;
    let micros = started.elapsed().as_micros();
    let psnr = if cli.print_psnr && !cli.quiet {
        let decoded = decode_webp(&encoded, &cli.output)?;
        Some(rgb_psnr(&image.pixels, &decoded.pixels))
    } else {
        None
    };
    write_output(&cli.output, &encoded)?;
    Ok(Success {
        width: image.width,
        height: image.height,
        byte_count: encoded.len(),
        micros,
        output: cli.output,
        quiet: cli.quiet,
        verbose: cli.verbose,
        psnr,
    })
}

fn rgb_psnr(source: &Pixels, decoded: &Pixels) -> f64 {
    let (source, source_channels) = match source {
        Pixels::Rgb(bytes) => (bytes, 3),
        Pixels::Rgba(bytes) => (bytes, 4),
    };
    let (decoded, decoded_channels) = match decoded {
        Pixels::Rgb(bytes) => (bytes, 3),
        Pixels::Rgba(bytes) => (bytes, 4),
    };
    let mut squared_error = 0_u64;
    for (source, decoded) in source
        .chunks_exact(source_channels)
        .zip(decoded.chunks_exact(decoded_channels))
    {
        for channel in 0..3 {
            let difference = i64::from(source[channel]) - i64::from(decoded[channel]);
            squared_error += (difference * difference) as u64;
        }
    }
    if squared_error == 0 {
        99.0
    } else {
        let samples = source.len() / source_channels * 3;
        let mean_squared_error = squared_error as f64 / samples as f64;
        10.0 * (255.0 * 255.0 / mean_squared_error).log10()
    }
}

fn read_input(input: &OsStr) -> Result<Vec<u8>, String> {
    if input == OsStr::new("-") {
        let mut bytes = Vec::new();
        std::io::stdin()
            .read_to_end(&mut bytes)
            .map_err(|_| READ_STDIN.to_owned())?;
        Ok(bytes)
    } else {
        std::fs::read(input).map_err(|_| named_problem(READ_PATH, input))
    }
}

fn decode_input(bytes: &[u8], input: &OsStr) -> Result<Image, String> {
    if bytes.starts_with(b"\x89PNG\r\n\x1a\n") {
        decode_png(bytes).map_err(|_| named_problem(PNG_ERROR, input))
    } else if bytes.starts_with(&[0xff, 0xd8]) {
        decode_jpeg(bytes, input)
    } else if bytes.len() >= 12 && &bytes[..4] == b"RIFF" && &bytes[8..12] == b"WEBP" {
        decode_webp(bytes, input)
    } else {
        Err(named_problem(UNSUPPORTED, input))
    }
}

fn decode_png(bytes: &[u8]) -> Result<Image, ()> {
    let mut decoder = png::Decoder::new(Cursor::new(bytes));
    decoder.set_transformations(png::Transformations::normalize_to_color8());
    let mut reader = decoder.read_info().map_err(|_| ())?;
    let size = reader.output_buffer_size().ok_or(())?;
    let mut pixels = vec![0; size];
    let info = reader.next_frame(&mut pixels).map_err(|_| ())?;
    pixels.truncate(info.buffer_size());
    let pixels = match info.color_type {
        png::ColorType::Rgb => Pixels::Rgb(pixels),
        png::ColorType::Rgba => Pixels::Rgba(pixels),
        png::ColorType::Grayscale => Pixels::Rgb(expand_gray(&pixels)),
        png::ColorType::GrayscaleAlpha => Pixels::Rgba(expand_gray_alpha(&pixels)),
        png::ColorType::Indexed => return Err(()),
    };
    Ok(Image {
        pixels,
        width: info.width,
        height: info.height,
    })
}

fn decode_jpeg(bytes: &[u8], input: &OsStr) -> Result<Image, String> {
    let mut decoder = jpeg_decoder::Decoder::new(Cursor::new(bytes));
    let pixels = decoder
        .decode()
        .map_err(|_| named_problem(JPEG_ERROR, input))?;
    let info = decoder
        .info()
        .ok_or_else(|| named_problem(JPEG_ERROR, input))?;
    let pixels = match info.pixel_format {
        jpeg_decoder::PixelFormat::L8 => Pixels::Rgb(expand_gray(&pixels)),
        jpeg_decoder::PixelFormat::RGB24 => Pixels::Rgb(pixels),
        jpeg_decoder::PixelFormat::CMYK32 => {
            return Err(named_problem(CMYK_ERROR, input));
        }
        _ => return Err(named_problem(JPEG_ERROR, input)),
    };
    Ok(Image {
        pixels,
        width: u32::from(info.width),
        height: u32::from(info.height),
    })
}

fn decode_webp(bytes: &[u8], input: &OsStr) -> Result<Image, String> {
    let mut decoder = image_webp::WebPDecoder::new(Cursor::new(bytes))
        .map_err(|_| named_problem(WEBP_ERROR, input))?;
    if decoder.is_animated() {
        return Err(named_problem(ANIMATED_ERROR, input));
    }
    let (width, height) = decoder.dimensions();
    let has_alpha = decoder.has_alpha();
    let channels = if has_alpha { 4 } else { 3 };
    let mut pixels = vec![0; width as usize * height as usize * channels];
    decoder
        .read_image(&mut pixels)
        .map_err(|_| named_problem(WEBP_ERROR, input))?;
    Ok(Image {
        pixels: if has_alpha {
            Pixels::Rgba(pixels)
        } else {
            Pixels::Rgb(pixels)
        },
        width,
        height,
    })
}

fn expand_gray(gray: &[u8]) -> Vec<u8> {
    let mut rgb = Vec::with_capacity(gray.len() * 3);
    for value in gray {
        rgb.extend_from_slice(&[*value; 3]);
    }
    rgb
}

fn expand_gray_alpha(gray_alpha: &[u8]) -> Vec<u8> {
    let mut rgba = Vec::with_capacity(gray_alpha.len() * 2);
    for pixel in gray_alpha.chunks_exact(2) {
        rgba.extend_from_slice(&[pixel[0], pixel[0], pixel[0], pixel[1]]);
    }
    rgba
}

fn write_output(output: &OsStr, bytes: &[u8]) -> Result<(), String> {
    if output == OsStr::new("-") {
        let mut stdout = std::io::stdout().lock();
        stdout
            .write_all(bytes)
            .and_then(|()| stdout.flush())
            .map_err(|_| WRITE_STDOUT.to_owned())
    } else {
        std::fs::write(Path::new(output), bytes).map_err(|_| named_problem(WRITE_PATH, output))
    }
}

fn display_path(path: &OsStr) -> String {
    let mut display = String::new();
    for character in path.to_string_lossy().chars() {
        if character.is_control() {
            display.extend(character.escape_default());
        } else {
            display.push(character);
        }
    }
    display
}

fn named_problem(template: &str, name: &OsStr) -> String {
    template.replace("{name}", &display_path(name))
}

fn summary(success: &Success) -> String {
    if success.verbose {
        VERBOSE_SUMMARY
            .replace("{width}", &success.width.to_string())
            .replace("{height}", &success.height.to_string())
            .replace("{bytes}", &success.byte_count.to_string())
            .replace("{ms}", &(success.micros / 1000).to_string())
            .replace("{micros}", &format!("{:03}", success.micros % 1000))
    } else {
        let output = if success.output == OsStr::new("-") {
            STDOUT_NAME.into()
        } else {
            display_path(&success.output)
        };
        SUMMARY
            .replace("{bytes}", &success.byte_count.to_string())
            .replace("{output}", &output)
    }
}
