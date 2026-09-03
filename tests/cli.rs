//! The command line input, output, and exit contracts.

#[path = "../fixtures/generator.rs"]
mod generator;
#[path = "../fixtures/png_writer.rs"]
mod png_writer;

use std::ffi::OsStr;
use std::io::{Cursor, Write};
use std::path::{Path, PathBuf};
use std::process::{Command, Output, Stdio};

use tiny_webp::{Alpha, Options};

fn command() -> Command {
    Command::new(env!("CARGO_BIN_EXE_tiny-webp"))
}

fn run<I, S>(args: I) -> Output
where
    I: IntoIterator<Item = S>,
    S: AsRef<OsStr>,
{
    command().args(args).output().expect("run the binary")
}

fn run_with_stdin<I, S>(args: I, input: &[u8]) -> Output
where
    I: IntoIterator<Item = S>,
    S: AsRef<OsStr>,
{
    let mut child = command()
        .args(args)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("start the binary");
    child
        .stdin
        .take()
        .expect("open the input pipe")
        .write_all(input)
        .expect("write the input pipe");
    child.wait_with_output().expect("collect the binary output")
}

fn readme_usage() -> String {
    let readme = std::fs::read_to_string(concat!(env!("CARGO_MANIFEST_DIR"), "/README.md"))
        .expect("read the README");
    let lines: Vec<&str> = readme.lines().collect();
    let start = lines
        .iter()
        .position(|line| *line == "usage: tiny-webp [options] <input> -o <output.webp>")
        .expect("find the usage block");
    let length = lines[start..]
        .iter()
        .position(|line| line.starts_with("```"))
        .expect("find the closing fence");
    let mut block = lines[start..start + length].join("\n");
    block.push('\n');
    block
}

fn assert_usage_error(args: &[&str], problem: &str) {
    let output = run(args);
    assert_eq!(output.status.code(), Some(2), "{args:?}");
    assert_eq!(output.stdout, b"", "{args:?}");
    let mut expected = String::from("tiny-webp: ");
    expected.push_str(problem);
    expected.push('\n');
    expected.push_str(&readme_usage());
    assert_eq!(
        String::from_utf8_lossy(&output.stderr),
        expected,
        "{args:?}"
    );
}

fn scratch_directory(test_name: &str) -> PathBuf {
    let directory =
        std::env::temp_dir().join(format!("tiny-webp-cli-{test_name}-{}", std::process::id()));
    if directory.exists() {
        std::fs::remove_dir_all(&directory).expect("remove the old test directory");
    }
    std::fs::create_dir_all(&directory).expect("create the test directory");
    directory
}

fn png_bytes(fixture: &generator::Fixture) -> Vec<u8> {
    let mut bytes = Vec::new();
    png_writer::write(
        &mut bytes,
        fixture.width,
        fixture.height,
        png::ColorType::Rgba,
        &fixture.rgba,
    )
    .expect("write the PNG pixels");
    bytes
}

fn png_with_color(width: u32, height: u32, color: png::ColorType, pixels: &[u8]) -> Vec<u8> {
    let mut bytes = Vec::new();
    png_writer::write(&mut bytes, width, height, color, pixels).expect("write the PNG pixels");
    bytes
}

fn lossless_webp_bytes(fixture: &generator::Fixture) -> Vec<u8> {
    let mut bytes = Vec::new();
    image_webp::WebPEncoder::new(&mut bytes)
        .encode(
            &fixture.rgba,
            fixture.width,
            fixture.height,
            image_webp::ColorType::Rgba8,
        )
        .expect("write the lossless WebP pixels");
    bytes
}

fn flag_rows() -> Vec<(Vec<&'static str>, Options)> {
    let mut q0 = Options::default();
    q0.quality = 0;
    let mut q50 = Options::default();
    q50.quality = 50;
    let mut q100 = Options::default();
    q100.quality = 100;
    let mut noalpha = Options::default();
    noalpha.alpha = Alpha::Discard;
    vec![
        (vec![], Options::default()),
        (vec!["-q", "0"], q0),
        (vec!["-q", "50"], q50),
        (vec!["-q", "100"], q100),
        (vec!["-noalpha"], noalpha),
    ]
}

fn run_file(input: &Path, output: &Path, flags: &[&str]) -> Output {
    let mut args: Vec<&OsStr> = flags.iter().map(OsStr::new).collect();
    args.push(input.as_os_str());
    args.push(OsStr::new("-o"));
    args.push(output.as_os_str());
    run(args)
}

#[test]
fn help_and_version_stop_parsing_when_the_parser_reaches_them() {
    let usage = readme_usage();
    for args in [
        vec!["-h"],
        vec!["--help"],
        vec!["-h", "--wat"],
        vec!["-h", "-version"],
    ] {
        let output = run(&args);
        assert_eq!(output.status.code(), Some(0), "{args:?}");
        assert_eq!(String::from_utf8_lossy(&output.stdout), usage, "{args:?}");
        assert_eq!(output.stderr, b"", "{args:?}");
    }
    for args in [vec!["-version"], vec!["--version"], vec!["-version", "-h"]] {
        let output = run(&args);
        assert_eq!(output.status.code(), Some(0), "{args:?}");
        assert_eq!(
            String::from_utf8_lossy(&output.stdout),
            concat!("tiny-webp ", env!("CARGO_PKG_VERSION"), "\n"),
            "{args:?}"
        );
        assert_eq!(output.stderr, b"", "{args:?}");
    }
    assert_usage_error(
        &["--wat", "-h"],
        "Unknown flag --wat. Expected a supported option.",
    );
}

#[test]
fn every_usage_error_prints_its_exact_problem_and_the_usage_text() {
    assert_usage_error(
        &["--noalpha=value"],
        "Unknown command line option. Expected a supported option.",
    );
    assert_usage_error(
        &["--sharp-yuv", "in.png", "-o", "out.webp"],
        "Unknown flag --sharp-yuv. Expected a supported option.",
    );
    assert_usage_error(
        &["-print_psnr", "in.png", "-o", "out.webp"],
        "Unknown flag --print_psnr. Expected a supported option.",
    );
    assert_usage_error(
        &["-f", "0", "in.png", "-o", "out.webp"],
        "Unknown flag -f. Expected a supported option.",
    );
    assert_usage_error(
        &["-sharpness", "0", "in.png", "-o", "out.webp"],
        "Unknown flag --sharpness. Expected a supported option.",
    );
    for quality in ["75.5", "101", "256", "-1", "high"] {
        assert_usage_error(
            &["-q", quality, "in.png", "-o", "out.webp"],
            &format!("Invalid quality {quality}. Expected a whole number from 0 to 100."),
        );
    }
    assert_usage_error(
        &["-o", "out.webp"],
        "Missing input path. Pass a file or - for stdin.",
    );
    assert_usage_error(
        &["in.png"],
        "Missing output path. Pass -o <file> or -o - for stdout.",
    );
    assert_usage_error(
        &["one.png", "two.png", "-o", "out.webp"],
        "Unexpected input path two.png. tiny-webp reads one input path.",
    );
    assert_usage_error(
        &["-q"],
        "Missing value for -q. Pass a quality integer between zero and one hundred.",
    );
    assert_usage_error(
        &["in.png", "-o"],
        "Missing value for -o. Expected an output path or -.",
    );
}

#[test]
fn repeated_flags_take_their_last_value_or_keep_their_first_effect() {
    let directory = scratch_directory("repeated_flags");
    let fixture = generator::all()
        .into_iter()
        .find(|fixture| fixture.name == "alpha-odd")
        .expect("find the alpha fixture");
    let input = directory.join("input.png");
    std::fs::write(&input, png_bytes(&fixture)).expect("write the PNG input");

    let repeated_q = directory.join("repeated-q.webp");
    let q90 = directory.join("q90.webp");
    assert_eq!(
        run_file(&input, &repeated_q, &["-q", "10", "-q", "90"])
            .status
            .code(),
        Some(0)
    );
    assert_eq!(run_file(&input, &q90, &["-q", "90"]).status.code(), Some(0));
    assert_eq!(
        std::fs::read(&repeated_q).expect("read the repeated quality output"),
        std::fs::read(&q90).expect("read the final quality output")
    );

    let first = directory.join("first.webp");
    let second = directory.join("second.webp");
    let output = run([
        input.as_os_str(),
        OsStr::new("-o"),
        first.as_os_str(),
        OsStr::new("-o"),
        second.as_os_str(),
    ]);
    assert_eq!(output.status.code(), Some(0));
    assert_eq!(u8::from(first.exists()), 0);
    assert_eq!(u8::from(second.exists()), 1);

    let one_word = directory.join("one-word.webp");
    let repeated_word = directory.join("repeated-word.webp");
    let one = run_file(&input, &one_word, &["-noalpha", "-quiet", "-v"]);
    let repeated = run_file(
        &input,
        &repeated_word,
        &["-noalpha", "-noalpha", "-quiet", "-quiet", "-v", "-v"],
    );
    assert_eq!(one.status.code(), Some(0));
    assert_eq!(repeated.status.code(), Some(0));
    assert_eq!(one.stdout, b"");
    assert_eq!(one.stderr, b"");
    assert_eq!(repeated.stdout, b"");
    assert_eq!(repeated.stderr, b"");
    assert_eq!(
        std::fs::read(one_word).expect("read the single word output"),
        std::fs::read(repeated_word).expect("read the repeated word output")
    );
    std::fs::remove_dir_all(directory).expect("remove the test directory");
}

#[test]
fn png_and_lossless_webp_inputs_match_the_library_for_every_fixture_and_flag_row() {
    let directory = scratch_directory("input_formats_match_library");
    for fixture in generator::all() {
        for (extension, bytes) in [
            ("png", png_bytes(&fixture)),
            ("webp", lossless_webp_bytes(&fixture)),
        ] {
            let input = directory.join(format!("{}.data", fixture.name));
            std::fs::write(&input, bytes).expect("write the image input");
            for (row, (flags, options)) in flag_rows().into_iter().enumerate() {
                let output_path =
                    directory.join(format!("{}-{extension}-{row}.webp", fixture.name));
                let output = run_file(&input, &output_path, &flags);
                assert_eq!(
                    output.status.code(),
                    Some(0),
                    "{} {extension} {row}",
                    fixture.name
                );
                let expected =
                    tiny_webp::encode_rgba(&fixture.rgba, fixture.width, fixture.height, &options)
                        .expect("encode the fixture through the library");
                assert_eq!(
                    std::fs::read(output_path).expect("read the command output"),
                    expected,
                    "{} {extension} {row}",
                    fixture.name
                );
            }
        }
    }
    std::fs::remove_dir_all(directory).expect("remove the test directory");
}

#[test]
fn baseline_jpeg_inputs_match_their_decoded_pixels_at_every_flag_row() {
    let directory = scratch_directory("jpeg_matches_library");
    let input = directory.join("constant.data");
    let jpeg = baseline_jpeg(1);
    std::fs::write(&input, &jpeg).expect("write the JPEG input");
    let mut decoder = jpeg_decoder::Decoder::new(Cursor::new(&jpeg));
    let gray = decoder.decode().expect("decode the test JPEG");
    assert_eq!(gray, vec![128; 64]);
    let rgb: Vec<u8> = gray.iter().flat_map(|value| [*value; 3]).collect();
    for (row, (flags, options)) in flag_rows().into_iter().enumerate() {
        let output_path = directory.join(format!("jpeg-{row}.webp"));
        let output = run_file(&input, &output_path, &flags);
        assert_eq!(output.status.code(), Some(0), "row {row}");
        assert_eq!(
            std::fs::read(output_path).expect("read the command output"),
            tiny_webp::encode_rgb(&rgb, 8, 8, &options).expect("encode the decoded JPEG pixels"),
            "row {row}"
        );
    }

    let color_input = directory.join("constant-color.data");
    let color_jpeg = baseline_jpeg(3);
    std::fs::write(&color_input, &color_jpeg).expect("write the color JPEG input");
    let mut decoder = jpeg_decoder::Decoder::new(Cursor::new(&color_jpeg));
    let rgb = decoder.decode().expect("decode the color test JPEG");
    assert_eq!(rgb, vec![128; 8 * 8 * 3]);
    for (row, (flags, options)) in flag_rows().into_iter().enumerate() {
        let output_path = directory.join(format!("color-jpeg-{row}.webp"));
        let output = run_file(&color_input, &output_path, &flags);
        assert_eq!(output.status.code(), Some(0), "color row {row}");
        assert_eq!(
            std::fs::read(output_path).expect("read the color command output"),
            tiny_webp::encode_rgb(&rgb, 8, 8, &options)
                .expect("encode the decoded color JPEG pixels"),
            "color row {row}"
        );
    }
    std::fs::remove_dir_all(directory).expect("remove the test directory");
}

#[test]
fn grayscale_png_inputs_expand_to_their_rgb_and_rgba_library_bytes() {
    let directory = scratch_directory("grayscale_png");
    let gray_input = directory.join("gray.png");
    let gray_output = directory.join("gray.webp");
    std::fs::write(
        &gray_input,
        png_with_color(2, 1, png::ColorType::Grayscale, &[64, 192]),
    )
    .expect("write the grayscale PNG");
    assert_eq!(
        run_file(&gray_input, &gray_output, &[]).status.code(),
        Some(0)
    );
    assert_eq!(
        std::fs::read(&gray_output).expect("read the grayscale output"),
        tiny_webp::encode_rgb(&[64, 64, 64, 192, 192, 192], 2, 1, &Options::default())
            .expect("encode the expanded grayscale pixels")
    );

    let alpha_input = directory.join("gray-alpha.png");
    let alpha_output = directory.join("gray-alpha.webp");
    std::fs::write(
        &alpha_input,
        png_with_color(2, 1, png::ColorType::GrayscaleAlpha, &[64, 7, 192, 255]),
    )
    .expect("write the grayscale alpha PNG");
    assert_eq!(
        run_file(&alpha_input, &alpha_output, &[]).status.code(),
        Some(0)
    );
    assert_eq!(
        std::fs::read(&alpha_output).expect("read the grayscale alpha output"),
        tiny_webp::encode_rgba(
            &[64, 64, 64, 7, 192, 192, 192, 255],
            2,
            1,
            &Options::default()
        )
        .expect("encode the expanded grayscale alpha pixels")
    );
    std::fs::remove_dir_all(directory).expect("remove the test directory");
}

#[test]
fn unreadable_and_unsupported_inputs_exit_one_with_one_exact_problem_line() {
    let directory = scratch_directory("input_problems");
    let output_path = directory.join("output.webp");
    let missing = directory.join("missing.png");
    let missing_output = run_file(&missing, &output_path, &[]);
    assert_eq!(missing_output.status.code(), Some(1));
    assert_eq!(missing_output.stdout, b"");
    assert_eq!(
        String::from_utf8_lossy(&missing_output.stderr),
        format!(
            "tiny-webp: Could not read {}. Check that the input path is readable.\n",
            missing.display()
        )
    );

    let text = directory.join("text.dat");
    std::fs::write(&text, b"plain text").expect("write the text input");
    let text_output = run_file(&text, &output_path, &[]);
    assert_eq!(text_output.status.code(), Some(1));
    assert_eq!(text_output.stdout, b"");
    assert_eq!(
        String::from_utf8_lossy(&text_output.stderr),
        format!(
            "tiny-webp: Could not decode {}. Expected PNG, JPEG, or WebP bytes.\n",
            text.display()
        )
    );

    let animated = directory.join("animated.webp");
    std::fs::write(&animated, animated_webp()).expect("write the animated WebP input");
    let animated_output = run_file(&animated, &output_path, &[]);
    assert_eq!(animated_output.status.code(), Some(1));
    assert_eq!(animated_output.stdout, b"");
    assert_eq!(
        String::from_utf8_lossy(&animated_output.stderr),
        format!(
            "tiny-webp: Could not decode {}. Animated WebP input is unsupported.\n",
            animated.display()
        )
    );

    let cmyk = directory.join("cmyk.jpg");
    std::fs::write(&cmyk, baseline_jpeg(4)).expect("write the CMYK JPEG input");
    let cmyk_output = run_file(&cmyk, &output_path, &[]);
    assert_eq!(cmyk_output.status.code(), Some(1));
    assert_eq!(cmyk_output.stdout, b"");
    assert_eq!(
        String::from_utf8_lossy(&cmyk_output.stderr),
        format!(
            "tiny-webp: Could not decode {}. CMYK JPEG input is unsupported.\n",
            cmyk.display()
        )
    );

    for (name, bytes, suffix) in [
        ("broken.png", b"\x89PNG\r\n\x1a\n".as_slice(), " as PNG."),
        ("broken.jpg", &[0xff, 0xd8], " as JPEG."),
        ("broken.webp", b"RIFF\0\0\0\0WEBP".as_slice(), " as WebP."),
    ] {
        let input = directory.join(name);
        std::fs::write(&input, bytes).expect("write the broken input");
        let broken_output = run_file(&input, &output_path, &[]);
        assert_eq!(broken_output.status.code(), Some(1), "{name}");
        assert_eq!(broken_output.stdout, b"", "{name}");
        assert_eq!(
            String::from_utf8_lossy(&broken_output.stderr),
            format!("tiny-webp: Could not decode {}{suffix}\n", input.display()),
            "{name}"
        );
    }
    std::fs::remove_dir_all(directory).expect("remove the test directory");
}

#[test]
fn an_oversized_decoded_image_exits_one_with_one_exact_problem_line() {
    let directory = scratch_directory("oversized_image");
    let input = directory.join("wide.png");
    let output_path = directory.join("output.webp");
    let pixels = vec![0; 16_384 * 3];
    std::fs::write(
        &input,
        png_with_color(16_384, 1, png::ColorType::Rgb, &pixels),
    )
    .expect("write the oversized PNG");
    let output = run_file(&input, &output_path, &[]);
    assert_eq!(output.status.code(), Some(1));
    assert_eq!(output.stdout, b"");
    assert_eq!(
        String::from_utf8_lossy(&output.stderr),
        format!(
            "tiny-webp: Could not encode {}. The decoded image dimensions are unsupported.\n",
            input.display()
        )
    );
    std::fs::remove_dir_all(directory).expect("remove the test directory");
}

#[test]
fn quiet_success_writes_neither_stream_for_a_file_output() {
    let directory = scratch_directory("quiet_success");
    let fixture = generator::all().remove(0);
    let input = directory.join("input.png");
    let output_path = directory.join("output.webp");
    std::fs::write(&input, png_bytes(&fixture)).expect("write the PNG input");
    let output = run_file(&input, &output_path, &["-quiet"]);
    assert_eq!(output.status.code(), Some(0));
    assert_eq!(output.stdout, b"");
    assert_eq!(output.stderr, b"");
    assert_eq!(u8::from(output_path.exists()), 1);
    std::fs::remove_dir_all(directory).expect("remove the test directory");
}

#[test]
fn stdin_and_stdout_write_the_same_bytes_as_file_paths() {
    let directory = scratch_directory("standard_streams");
    let fixture = generator::all().remove(0);
    let png = png_bytes(&fixture);
    let input = directory.join("input.png");
    let output_path = directory.join("output.webp");
    std::fs::write(&input, &png).expect("write the PNG input");
    let file_output = run_file(&input, &output_path, &["-quiet"]);
    assert_eq!(file_output.status.code(), Some(0));
    let stream_output = run_with_stdin(["-quiet", "-", "-o", "-"], &png);
    assert_eq!(stream_output.status.code(), Some(0));
    assert_eq!(stream_output.stderr, b"");
    assert_eq!(
        stream_output.stdout,
        std::fs::read(output_path).expect("read the file output")
    );
    std::fs::remove_dir_all(directory).expect("remove the test directory");
}

#[cfg(unix)]
#[test]
fn an_unreadable_stdin_exits_one_with_its_exact_problem_line() {
    let directory = scratch_directory("stdin_problem");
    let output_path = directory.join("output.webp");
    let stdin = std::fs::File::open(&directory).expect("open the test directory");
    let output = command()
        .args([OsStr::new("-"), OsStr::new("-o"), output_path.as_os_str()])
        .stdin(stdin)
        .output()
        .expect("run the binary");
    assert_eq!(output.status.code(), Some(1));
    assert_eq!(output.stdout, b"");
    assert_eq!(
        output.stderr,
        b"tiny-webp: Could not read stdin. Check that standard input is readable.\n"
    );
    std::fs::remove_dir_all(directory).expect("remove the test directory");
}

#[cfg(unix)]
#[test]
fn an_unwritable_stdout_exits_one_with_its_exact_problem_line() {
    let fixture = generator::all().remove(0);
    let mut child = command()
        .args(["-quiet", "-", "-o", "-"])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("start the binary");
    drop(child.stdout.take().expect("open the output pipe"));
    child
        .stdin
        .take()
        .expect("open the input pipe")
        .write_all(&png_bytes(&fixture))
        .expect("write the input pipe");
    let output = child.wait_with_output().expect("collect the binary output");
    assert_eq!(output.status.code(), Some(1));
    assert_eq!(output.stdout, b"");
    assert_eq!(
        output.stderr,
        b"tiny-webp: Could not write stdout. Check that standard output is writable.\n"
    );
}

#[test]
fn summary_and_verbose_success_lines_match_their_exact_shapes() {
    let directory = scratch_directory("success_lines");
    let fixture = generator::all().remove(0);
    let input = directory.join("input.png");
    let output_path = directory.join("output.webp");
    std::fs::write(&input, png_bytes(&fixture)).expect("write the PNG input");
    let output = run_file(&input, &output_path, &[]);
    let byte_count = std::fs::read(&output_path)
        .expect("read the file output")
        .len();
    assert_eq!(output.status.code(), Some(0));
    assert_eq!(output.stdout, b"");
    assert_eq!(
        String::from_utf8_lossy(&output.stderr),
        format!(
            "tiny-webp: wrote {byte_count} bytes to {}\n",
            output_path.display()
        )
    );

    let verbose = run_file(&input, &output_path, &["-v"]);
    let line = String::from_utf8(verbose.stderr).expect("read the verbose line");
    let prefix = format!("tiny-webp: 32x32, {byte_count} bytes, ");
    assert_eq!(verbose.status.code(), Some(0));
    assert_eq!(verbose.stdout, b"");
    assert_eq!(u8::from(line.starts_with(&prefix)), 1);
    assert_eq!(u8::from(line.ends_with(" ms\n")), 1);
    let time = &line[prefix.len()..line.len() - 4];
    let parts: Vec<&str> = time.split('.').collect();
    assert_eq!(parts.len(), 2);
    assert_eq!(
        u8::from(parts[0].chars().all(|value| value.is_ascii_digit())),
        1
    );
    assert_eq!(parts[1].len(), 3);
    assert_eq!(
        u8::from(parts[1].chars().all(|value| value.is_ascii_digit())),
        1
    );
    std::fs::remove_dir_all(directory).expect("remove the test directory");
}

#[test]
fn an_unwritable_output_exits_one_with_one_exact_problem_line() {
    let directory = scratch_directory("output_problem");
    let fixture = generator::all().remove(0);
    let input = directory.join("input.png");
    std::fs::write(&input, png_bytes(&fixture)).expect("write the PNG input");
    let output = run_file(&input, &directory, &[]);
    assert_eq!(output.status.code(), Some(1));
    assert_eq!(output.stdout, b"");
    assert_eq!(
        String::from_utf8_lossy(&output.stderr),
        format!(
            "tiny-webp: Could not write {}. Check that the output path is writable.\n",
            directory.display()
        )
    );
    std::fs::remove_dir_all(directory).expect("remove the test directory");
}

#[cfg(unix)]
#[test]
fn a_non_utf8_input_path_reaches_the_same_rewrite_and_names_the_path_lossily() {
    use std::os::unix::ffi::OsStringExt;

    let directory = scratch_directory("non_utf8_path");
    let mut bytes = directory.as_os_str().as_encoded_bytes().to_vec();
    bytes.push(b'/');
    bytes.extend_from_slice(b"image-");
    bytes.push(0xff);
    bytes.extend_from_slice(b".png");
    let input = PathBuf::from(std::ffi::OsString::from_vec(bytes));
    let output_path = directory.join("output.webp");
    let output = run_file(&input, &output_path, &["-noalpha"]);
    assert_eq!(output.status.code(), Some(1));
    assert_eq!(output.stdout, b"");
    assert_eq!(
        String::from_utf8_lossy(&output.stderr),
        format!(
            "tiny-webp: Could not read {}. Check that the input path is readable.\n",
            input.to_string_lossy()
        )
    );
    std::fs::remove_dir_all(directory).expect("remove the test directory");
}

fn animated_webp() -> Vec<u8> {
    let still = tiny_webp::encode_rgb(&[96, 128, 160], 1, 1, &Options::default())
        .expect("encode the animation frame");
    let frame_chunk = &still[12..];
    let mut webp = Vec::new();
    webp.extend_from_slice(b"RIFF");
    webp.extend_from_slice(&[0; 4]);
    webp.extend_from_slice(b"WEBP");
    write_chunk(&mut webp, b"VP8X", &[0x02, 0, 0, 0, 0, 0, 0, 0, 0, 0]);
    write_chunk(&mut webp, b"ANIM", &[0; 6]);
    for _ in 0..2 {
        let mut frame = vec![0; 16];
        frame[12] = 1;
        frame.extend_from_slice(frame_chunk);
        write_chunk(&mut webp, b"ANMF", &frame);
    }
    let size = (webp.len() - 8) as u32;
    webp[4..8].copy_from_slice(&size.to_le_bytes());
    webp
}

fn write_chunk(output: &mut Vec<u8>, name: &[u8; 4], payload: &[u8]) {
    output.extend_from_slice(name);
    output.extend_from_slice(&(payload.len() as u32).to_le_bytes());
    output.extend_from_slice(payload);
    if payload.len() % 2 == 1 {
        output.push(0);
    }
}

fn baseline_jpeg(component_count: u8) -> Vec<u8> {
    let mut jpeg = vec![0xff, 0xd8];
    if component_count == 4 {
        push_jpeg_segment(
            &mut jpeg,
            0xee,
            &[b'A', b'd', b'o', b'b', b'e', 0, 100, 0, 0, 0, 0, 0],
        );
    }

    let mut quantization = vec![0];
    quantization.extend_from_slice(&[
        16, 11, 12, 14, 12, 10, 16, 14, 13, 14, 18, 17, 16, 19, 24, 40, 26, 24, 22, 22, 24, 49, 35,
        37, 29, 40, 58, 51, 61, 60, 57, 51, 56, 55, 64, 72, 92, 78, 64, 68, 87, 69, 55, 56, 80,
        109, 81, 87, 95, 98, 103, 104, 103, 62, 77, 113, 121, 112, 100, 120, 92, 101, 103, 99,
    ]);
    push_jpeg_segment(&mut jpeg, 0xdb, &quantization);

    let mut frame = vec![8, 0, 8, 0, 8, component_count];
    for component in 1..=component_count {
        frame.extend_from_slice(&[component, 0x11, 0]);
    }
    push_jpeg_segment(&mut jpeg, 0xc0, &frame);

    let mut dc = vec![0x00];
    dc.extend_from_slice(&[0, 1, 5, 1, 1, 1, 1, 1, 1, 0, 0, 0, 0, 0, 0, 0]);
    dc.extend(0..=11);
    push_jpeg_segment(&mut jpeg, 0xc4, &dc);

    let mut ac = vec![0x10];
    ac.extend_from_slice(&[0, 2, 1, 3, 3, 2, 4, 3, 5, 5, 4, 4, 0, 0, 1, 0x7d]);
    ac.extend_from_slice(&[
        0x01, 0x02, 0x03, 0x00, 0x04, 0x11, 0x05, 0x12, 0x21, 0x31, 0x41, 0x06, 0x13, 0x51, 0x61,
        0x07, 0x22, 0x71, 0x14, 0x32, 0x81, 0x91, 0xa1, 0x08, 0x23, 0x42, 0xb1, 0xc1, 0x15, 0x52,
        0xd1, 0xf0, 0x24, 0x33, 0x62, 0x72, 0x82, 0x09, 0x0a, 0x16, 0x17, 0x18, 0x19, 0x1a, 0x25,
        0x26, 0x27, 0x28, 0x29, 0x2a, 0x34, 0x35, 0x36, 0x37, 0x38, 0x39, 0x3a, 0x43, 0x44, 0x45,
        0x46, 0x47, 0x48, 0x49, 0x4a, 0x53, 0x54, 0x55, 0x56, 0x57, 0x58, 0x59, 0x5a, 0x63, 0x64,
        0x65, 0x66, 0x67, 0x68, 0x69, 0x6a, 0x73, 0x74, 0x75, 0x76, 0x77, 0x78, 0x79, 0x7a, 0x83,
        0x84, 0x85, 0x86, 0x87, 0x88, 0x89, 0x8a, 0x92, 0x93, 0x94, 0x95, 0x96, 0x97, 0x98, 0x99,
        0x9a, 0xa2, 0xa3, 0xa4, 0xa5, 0xa6, 0xa7, 0xa8, 0xa9, 0xaa, 0xb2, 0xb3, 0xb4, 0xb5, 0xb6,
        0xb7, 0xb8, 0xb9, 0xba, 0xc2, 0xc3, 0xc4, 0xc5, 0xc6, 0xc7, 0xc8, 0xc9, 0xca, 0xd2, 0xd3,
        0xd4, 0xd5, 0xd6, 0xd7, 0xd8, 0xd9, 0xda, 0xe1, 0xe2, 0xe3, 0xe4, 0xe5, 0xe6, 0xe7, 0xe8,
        0xe9, 0xea, 0xf1, 0xf2, 0xf3, 0xf4, 0xf5, 0xf6, 0xf7, 0xf8, 0xf9, 0xfa,
    ]);
    push_jpeg_segment(&mut jpeg, 0xc4, &ac);

    let mut scan = vec![component_count];
    for component in 1..=component_count {
        scan.extend_from_slice(&[component, 0]);
    }
    scan.extend_from_slice(&[0, 63, 0]);
    push_jpeg_segment(&mut jpeg, 0xda, &scan);
    let mut bits = Vec::new();
    for _ in 0..component_count {
        bits.extend_from_slice(&[false, false, true, false, true, false]);
    }
    while bits.len() % 8 != 0 {
        bits.push(true);
    }
    for byte_bits in bits.chunks_exact(8) {
        let mut byte = 0u8;
        for bit in byte_bits {
            byte = (byte << 1) | u8::from(*bit);
        }
        jpeg.push(byte);
        if byte == 0xff {
            jpeg.push(0);
        }
    }
    jpeg.extend_from_slice(&[0xff, 0xd9]);
    jpeg
}

fn push_jpeg_segment(jpeg: &mut Vec<u8>, marker: u8, payload: &[u8]) {
    jpeg.extend_from_slice(&[0xff, marker]);
    jpeg.extend_from_slice(&((payload.len() + 2) as u16).to_be_bytes());
    jpeg.extend_from_slice(payload);
}
