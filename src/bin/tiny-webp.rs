//! The `tiny-webp` command line.

use std::ffi::{OsStr, OsString};
use std::io::{Cursor, Read, Write};
use std::path::Path;
use std::process::ExitCode;
use std::time::Instant;

use lexopt::prelude::{Long, Short, Value};
use tiny_webp::{Alpha, Options};

const USAGE: &str = "\
usage: tiny-webp [options] <input> -o <output.webp>

  -q <0..100>, --quality <0..100>   quality, default 75
  -o <file>,   --output <file>      output path, or - for stdout
  -noalpha                          drop the alpha plane
  -quiet                            no output on success
  -v                                print dimensions, bytes, and encode time
  -version, --version
  -h, --help
";
const PROGRAM_PREFIX: &str = "tiny-webp: ";
const VERSION_PREFIX: &str = "tiny-webp ";
const LONG_FLAG_PREFIX: &str = "--";
const SHORT_FLAG_PREFIX: &str = "-";
const STDOUT_NAME: &str = "stdout";
const UNKNOWN_FLAG: &str = "Unknown flag {flag}. Expected a supported option.";
const UNKNOWN_OPTION: &str = "Unknown command line option. Expected a supported option.";
const INVALID_QUALITY: &str = "Invalid quality {quality}. Expected a whole number from 0 to 100.";
const MISSING_QUALITY: &str =
    "Missing value for -q. Pass a quality integer between zero and one hundred.";
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
const ENCODE_ERROR: &str = "Could not encode {name}. The decoded image dimensions are unsupported.";
const WRITE_PATH: &str = "Could not write {name}. Check that the output path is writable.";
const WRITE_STDOUT: &str = "Could not write stdout. Check that standard output is writable.";
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
}

fn main() -> ExitCode {
    match parse(std::env::args_os().skip(1)) {
        Ok(Action::Help) => {
            print!("{USAGE}");
            ExitCode::SUCCESS
        }
        Ok(Action::Version) => {
            println!("{VERSION_PREFIX}{}", env!("CARGO_PKG_VERSION"));
            ExitCode::SUCCESS
        }
        Ok(Action::Encode(cli)) => match encode(cli) {
            Ok(success) => {
                if !success.quiet {
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

fn parse<I>(args: I) -> Result<Action, String>
where
    I: IntoIterator<Item = OsString>,
{
    let mut input: Option<OsString> = None;
    let mut output: Option<OsString> = None;
    let mut options = Options::default();
    let mut quiet = false;
    let mut verbose = false;

    let args = args.into_iter().map(expand_single_dash);
    let mut parser = lexopt::Parser::from_args(args);
    while let Some(arg) = parser.next().map_err(|_| UNKNOWN_OPTION.to_owned())? {
        match arg {
            Short('h') | Long("help") => return Ok(Action::Help),
            Long("version") => return Ok(Action::Version),
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
            Long("noalpha") => options.alpha = Alpha::Discard,
            Long("quiet") => quiet = true,
            Short('v') => verbose = true,
            Long(flag) => return Err(unknown_long_flag(flag)),
            Short(flag) => return Err(unknown_short_flag(flag)),
            Value(path) => {
                if input.replace(path.clone()).is_some() {
                    return Err(named_problem(SECOND_INPUT, &path));
                }
            }
        }
    }

    let input = input.ok_or_else(|| MISSING_INPUT.to_owned())?;
    let output = output.ok_or_else(|| MISSING_OUTPUT.to_owned())?;
    Ok(Action::Encode(Cli {
        input,
        output,
        options,
        quiet,
        verbose,
    }))
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

fn unknown_long_flag(flag: &str) -> String {
    let mut name = String::from(LONG_FLAG_PREFIX);
    name.push_str(flag);
    UNKNOWN_FLAG.replace("{flag}", &name)
}

fn unknown_short_flag(flag: char) -> String {
    let mut name = String::from(SHORT_FLAG_PREFIX);
    name.push(flag);
    UNKNOWN_FLAG.replace("{flag}", &name)
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
    write_output(&cli.output, &encoded)?;
    Ok(Success {
        width: image.width,
        height: image.height,
        byte_count: encoded.len(),
        micros,
        output: cli.output,
        quiet: cli.quiet,
        verbose: cli.verbose,
    })
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

fn named_problem(template: &str, name: &OsStr) -> String {
    template.replace("{name}", &name.to_string_lossy())
}

fn summary(success: &Success) -> String {
    if success.verbose {
        VERBOSE_SUMMARY
            .replace("{width}", &success.width.to_string())
            .replace("{height}", &success.height.to_string())
            .replace("{bytes}", &success.byte_count.to_string())
            .replace("{ms}", &(success.micros / 1000).to_string())
            .replace("{micros}", &three_digits((success.micros % 1000) as u16))
    } else {
        let output = if success.output == OsStr::new("-") {
            STDOUT_NAME.into()
        } else {
            success.output.to_string_lossy()
        };
        SUMMARY
            .replace("{bytes}", &success.byte_count.to_string())
            .replace("{output}", &output)
    }
}

fn three_digits(value: u16) -> String {
    let mut output = String::with_capacity(3);
    output.push(char::from(b'0' + (value / 100) as u8));
    output.push(char::from(b'0' + ((value / 10) % 10) as u8));
    output.push(char::from(b'0' + (value % 10) as u8));
    output
}
