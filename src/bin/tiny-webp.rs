//! The `tiny-webp` command line.

use std::ffi::{OsStr, OsString};
use std::process::ExitCode;

use lexopt::prelude::{Long, Short, Value};
use tiny_webp::{Alpha, Error, Options};

/// The help text, printed by `-h` and after every usage error.
///
/// The README carries this block verbatim and a test holds the two together.
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

/// What a command line asked for.
enum Action {
    Help,
    Version,
    Encode(Cli),
}

/// A command line that named an input, an output, and encoder settings.
struct Cli {
    input: OsString,
    output: OsString,
    options: Options,
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
            println!("tiny-webp {}", env!("CARGO_PKG_VERSION"));
            ExitCode::SUCCESS
        }
        Ok(Action::Encode(cli)) => {
            let Cli {
                input,
                output,
                options,
                quiet,
                verbose,
            } = cli;
            // The 0.1.0 encoder opens these paths and reads these flags. At
            // 0.0.0 the run stops here, before touching the file system.
            let _ = (input, output, options, quiet, verbose);
            eprintln!("tiny-webp: {}", Error::Unimplemented);
            ExitCode::from(1)
        }
        Err(problem) => {
            eprintln!("tiny-webp: {problem}");
            eprint!("{USAGE}");
            ExitCode::from(2)
        }
    }
}

/// Reads a command line without the binary name.
///
/// The error is one line naming the problem, which the caller prints ahead of
/// the usage text.
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
    while let Some(arg) = parser.next().map_err(|err| err.to_string())? {
        match arg {
            Short('h') | Long("help") => return Ok(Action::Help),
            Long("version") => return Ok(Action::Version),
            Short('q') | Long("quality") => {
                let raw = parser.value().map_err(|err| err.to_string())?;
                options.quality = parse_quality(&raw)?;
            }
            Short('o') | Long("output") => {
                let path = parser.value().map_err(|err| err.to_string())?;
                if output.replace(path).is_some() {
                    return Err("-o came twice, tiny-webp writes one output".into());
                }
            }
            Long("noalpha") => options.alpha = Alpha::Discard,
            Long("quiet") => quiet = true,
            Short('v') => verbose = true,
            Value(path) => {
                if input.replace(path).is_some() {
                    return Err("tiny-webp reads one input path".into());
                }
            }
            other => return Err(other.unexpected().to_string()),
        }
    }

    let input = input.ok_or("an input path is missing, pass a file or - for stdin")?;
    let output = output.ok_or("an output path is missing, pass -o <file> or -o - for stdout")?;

    Ok(Action::Encode(Cli {
        input,
        output,
        options,
        quiet,
        verbose,
    }))
}

/// Rewrites cwebp's one-dash word flags into the form lexopt reads.
///
/// lexopt reads a single dash as a cluster of short flags, so `-noalpha`
/// would come apart into seven of them. Any argument that opens on one dash
/// and runs past two characters gains a second dash here, which makes
/// `-noalpha` and `--noalpha` the same flag. A bare `-` stays a value and
/// means stdin or stdout.
fn expand_single_dash(arg: OsString) -> OsString {
    match arg.to_str() {
        Some(text) if text.len() > 2 && text.starts_with('-') && !text.starts_with("--") => {
            let mut expanded = String::with_capacity(text.len() + 1);
            expanded.push('-');
            expanded.push_str(text);
            OsString::from(expanded)
        }
        _ => arg,
    }
}

/// Reads the value of `-q`.
///
/// # Errors
///
/// Returns a line naming the accepted form for a fractional value, a value
/// past 100, or anything that is not a number.
fn parse_quality(raw: &OsStr) -> Result<u8, String> {
    let text = raw.to_string_lossy();
    match text.parse::<u8>() {
        Ok(quality) if quality <= 100 => Ok(quality),
        _ => Err(format!(
            "{text} is not a quality, -q takes a whole number from 0 to 100"
        )),
    }
}
