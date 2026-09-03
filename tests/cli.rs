//! The exit codes and the streams the binary writes.

use std::process::{Command, Output};

/// The one line the binary writes when a command line parses cleanly.
const UNIMPLEMENTED_LINE: &str = "tiny-webp: encoding is not implemented in 0.0.0\n";

/// Runs the binary with `args` and collects both streams.
fn run(args: &[&str]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_tiny-webp"))
        .args(args)
        .output()
        .expect("the binary runs")
}

/// Reads the fenced usage block out of the README.
fn readme_usage() -> String {
    let readme = std::fs::read_to_string(concat!(env!("CARGO_MANIFEST_DIR"), "/README.md"))
        .expect("the README sits beside the manifest");
    let lines: Vec<&str> = readme.lines().collect();
    let start = lines
        .iter()
        .position(|line| *line == "usage: tiny-webp [options] <input> -o <output.webp>")
        .expect("the README carries the usage block");
    let length = lines[start..]
        .iter()
        .position(|line| line.starts_with("```"))
        .expect("the usage block is fenced");
    let mut block = lines[start..start + length].join("\n");
    block.push('\n');
    block
}

/// Holds the usage-error contract for one command line.
fn usage_error(args: &[&str]) {
    let out = run(args);
    assert_eq!(out.status.code(), Some(2), "{args:?}");
    assert_eq!(String::from_utf8_lossy(&out.stdout), "", "{args:?}");

    let stderr = String::from_utf8_lossy(&out.stderr).into_owned();
    let (problem, usage) = stderr
        .split_once('\n')
        .expect("stderr carries a problem line ahead of the usage text");
    assert_eq!(problem.get(..11), Some("tiny-webp: "), "{args:?}");
    assert_eq!(usage, readme_usage(), "{args:?}");
}

#[test]
fn help_prints_the_usage_text_on_stdout_and_exits_zero() {
    for flag in ["-h", "--help"] {
        let out = run(&[flag]);
        assert_eq!(out.status.code(), Some(0), "{flag}");
        assert_eq!(
            String::from_utf8_lossy(&out.stdout),
            readme_usage(),
            "{flag}"
        );
        assert_eq!(String::from_utf8_lossy(&out.stderr), "", "{flag}");
    }
}

#[test]
fn version_prints_the_crate_version_on_stdout_and_exits_zero() {
    for flag in ["-version", "--version"] {
        let out = run(&[flag]);
        assert_eq!(out.status.code(), Some(0), "{flag}");
        assert_eq!(
            String::from_utf8_lossy(&out.stdout),
            "tiny-webp 0.0.0\n",
            "{flag}"
        );
        assert_eq!(String::from_utf8_lossy(&out.stderr), "", "{flag}");
    }
}

#[test]
fn an_unknown_flag_is_a_usage_error() {
    usage_error(&["--sharp-yuv", "in.png", "-o", "out.webp"]);
    usage_error(&["-print_psnr", "in.png", "-o", "out.webp"]);
}

#[test]
fn a_quality_outside_the_whole_numbers_from_zero_to_one_hundred_is_a_usage_error() {
    for quality in ["75.5", "101", "256", "-1", "high"] {
        usage_error(&["-q", quality, "in.png", "-o", "out.webp"]);
    }
}

#[test]
fn a_missing_input_or_a_missing_output_is_a_usage_error() {
    usage_error(&["-o", "out.webp"]);
    usage_error(&["in.png"]);
}

#[test]
fn a_second_positional_is_a_usage_error() {
    usage_error(&["one.png", "two.png", "-o", "out.webp"]);
}

#[test]
fn noalpha_reads_the_same_with_one_dash_and_with_two() {
    for flag in ["-noalpha", "--noalpha"] {
        let out = run(&[flag, "in.png", "-o", "out.webp"]);
        assert_eq!(out.status.code(), Some(1), "{flag}");
        assert_eq!(
            String::from_utf8_lossy(&out.stderr),
            UNIMPLEMENTED_LINE,
            "{flag}"
        );
    }
}

#[test]
fn a_well_formed_command_line_exits_one_with_one_line_on_stderr_before_opening_a_file() {
    let out = run(&[
        "/no/such/directory/input.png",
        "-o",
        "/no/such/directory/output.webp",
    ]);
    assert_eq!(out.status.code(), Some(1));
    assert_eq!(String::from_utf8_lossy(&out.stdout), "");
    assert_eq!(String::from_utf8_lossy(&out.stderr), UNIMPLEMENTED_LINE);
}

#[test]
fn quiet_leaves_the_unimplemented_line_in_place() {
    let out = run(&["-quiet", "-q", "80", "in.png", "-o", "-"]);
    assert_eq!(out.status.code(), Some(1));
    assert_eq!(String::from_utf8_lossy(&out.stdout), "");
    assert_eq!(String::from_utf8_lossy(&out.stderr), UNIMPLEMENTED_LINE);
}

#[test]
fn a_bare_dash_stays_a_value_on_the_input_and_on_the_output() {
    let out = run(&["-v", "-", "-o", "-"]);
    assert_eq!(out.status.code(), Some(1));
    assert_eq!(String::from_utf8_lossy(&out.stderr), UNIMPLEMENTED_LINE);
}
