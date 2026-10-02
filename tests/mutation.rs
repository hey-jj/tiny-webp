#![forbid(unsafe_code)]

use std::fs::{self, File};
use std::io::Cursor;
use std::panic::{catch_unwind, AssertUnwindSafe};
use std::path::{Path, PathBuf};

#[path = "support/mutation.rs"]
mod mutation;

use mutation::{corpus, outcome, read_replay, write_replay, Input, Mutation, Rng};

const SEED: &str = "mutation-runner";
const DEFAULT_RUNS: usize = 256;

fn budget(value: Option<&str>) -> Result<usize, String> {
    match value {
        None => Ok(DEFAULT_RUNS),
        Some(value) => value
            .parse()
            .map_err(|_| "TINY_WEBP_MUTATION_RUNS must be an unsigned integer.".to_owned()),
    }
}

fn failure_directory() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/mutation-failures")
}

fn check_input(input: &Input) -> Result<(), String> {
    let encoded = tiny_webp::encode_rgba(&input.pixels, input.width, input.height, &input.options);
    let actual = encoded.as_ref().map(|_| ()).map_err(Clone::clone);
    let expected = outcome(input);
    if actual != expected {
        return Err(format!("Expected {expected:?}, received {actual:?}."));
    }
    if let Ok(bytes) = encoded {
        let mut decoder = image_webp::WebPDecoder::new(Cursor::new(bytes))
            .map_err(|error| format!("Decode header failed: {error}."))?;
        let dimensions = decoder.dimensions();
        if dimensions != (input.width, input.height) {
            return Err(format!(
                "Expected dimensions {}x{}, received {}x{}.",
                input.width, input.height, dimensions.0, dimensions.1
            ));
        }
        let channels = if decoder.has_alpha() { 4 } else { 3 };
        let mut pixels = vec![0; input.width as usize * input.height as usize * channels];
        decoder
            .read_image(&mut pixels)
            .map_err(|error| format!("Decode pixels failed: {error}."))?;
    }
    Ok(())
}

fn checked(
    input: &Input,
    extra_check: &mut impl FnMut(&Input) -> Result<(), String>,
) -> Result<(), String> {
    catch_unwind(AssertUnwindSafe(|| {
        check_input(input)?;
        extra_check(input)
    }))
    .unwrap_or_else(|panic| {
        let message = panic
            .downcast_ref::<String>()
            .map(String::as_str)
            .or_else(|| panic.downcast_ref::<&str>().copied())
            .unwrap_or("Panic payload has no text.");
        Err(format!("Check panicked: {message}"))
    })
}

fn run_mutations(
    seed: &str,
    runs: usize,
    entries: &[Input],
    directory: &Path,
    mut extra_check: impl FnMut(&Input) -> Result<(), String>,
) -> Result<usize, String> {
    let mut rng = Rng::seeded(seed);
    for run in 0..runs {
        let mut input = entries[rng.below(entries.len() as u32) as usize].clone();
        for _ in 0..1 + rng.below(4) {
            Mutation::draw(&mut rng).apply(&mut input, &mut rng);
        }
        if let Err(problem) = checked(&input, &mut extra_check) {
            let path = directory.join(format!("{seed}-{run}.replay"));
            let report = format!(
                "Seed {seed}, run {run}, expected {:?}: {problem} Replay: {}",
                outcome(&input),
                path.display()
            );
            let save = || -> std::io::Result<()> {
                fs::create_dir_all(directory)?;
                write_replay(&input, File::create(&path)?)
            };
            save().map_err(|error| format!("{report}. Writing replay failed: {error}."))?;
            return Err(report);
        }
    }
    Ok(runs)
}

fn replay(
    directory: &Path,
    mut extra_check: impl FnMut(&Input) -> Result<(), String>,
) -> Result<usize, String> {
    let mut paths = fs::read_dir(directory)
        .map_err(|error| format!("Read replay directory failed: {error}."))?
        .map(|entry| entry.map(|entry| entry.path()))
        .collect::<Result<Vec<_>, _>>()
        .map_err(|error| format!("Read replay entry failed: {error}."))?;
    paths.retain(|path| path.file_name().is_none_or(|name| name != ".gitkeep"));
    paths.sort();
    let mut failures = Vec::new();
    for path in &paths {
        let result = File::open(path)
            .and_then(read_replay)
            .map_err(|error| format!("Read replay failed: {error}."))
            .and_then(|input| {
                checked(&input, &mut extra_check)
                    .map_err(|error| format!("Expected {:?}: {error}", outcome(&input)))
            });
        if let Err(problem) = result {
            failures.push(format!("{}: {problem}", path.display()));
        }
    }
    if failures.is_empty() {
        Ok(paths.len())
    } else {
        Err(failures.join("\n"))
    }
}

fn scratch(test_name: &str) -> PathBuf {
    let directory =
        std::env::temp_dir().join(format!("tiny-webp-{test_name}-{}", std::process::id()));
    fs::create_dir_all(&directory).unwrap();
    directory
}

#[test]
fn the_budget_defaults_to_256_and_accepts_an_unsigned_run_count() {
    assert_eq!(budget(None), Ok(256));
    assert_eq!(budget(Some("200000")), Ok(200_000));
    assert_eq!(budget(Some("0")), Ok(0));
    for value in ["", "-1", "1.5", "abc", "18446744073709551616"] {
        assert_eq!(
            budget(Some(value)),
            Err("TINY_WEBP_MUTATION_RUNS must be an unsigned integer.".to_owned())
        );
    }
}

#[test]
fn mutated_inputs_match_the_expected_outcome_and_decode_at_the_input_dimensions() {
    let value = std::env::var("TINY_WEBP_MUTATION_RUNS").ok();
    let runs = budget(value.as_deref()).expect("read the mutation budget");
    assert_eq!(
        run_mutations(SEED, runs, &corpus(), &failure_directory(), |_| Ok(())),
        Ok(runs)
    );
}

#[test]
fn saved_inputs_pass_every_check_when_replayed() {
    let directory = failure_directory();
    let files = fs::read_dir(&directory)
        .unwrap()
        .map(|entry| entry.unwrap().file_name())
        .filter(|name| name != ".gitkeep")
        .count();
    assert_eq!(replay(&directory, |_| Ok(())), Ok(files));
}

#[test]
fn the_same_seed_and_budget_visit_the_same_inputs() {
    use sha2::{Digest, Sha256};

    let entries = corpus();
    let mut visits = Vec::new();
    for _ in 0..2 {
        let mut digests = Vec::new();
        assert_eq!(
            run_mutations(SEED, 32, &entries, &failure_directory(), |input| {
                let mut bytes = Vec::new();
                write_replay(input, &mut bytes).unwrap();
                digests.push(Sha256::digest(bytes));
                Ok(())
            }),
            Ok(32)
        );
        visits.push(digests);
    }
    assert_eq!(visits[0].len(), 32);
    assert_eq!(visits[0], visits[1]);
}

#[test]
fn a_failed_check_saves_the_input_and_replay_reproduces_the_failure() {
    let directory = scratch("a_failed_check_saves_the_input_and_replay_reproduces_the_failure");
    let mut saved = None;
    let result = run_mutations(SEED, 1, &corpus(), &directory, |input| {
        saved = Some(input.clone());
        Err("Injected check failed.".to_owned())
    });
    let input = saved.unwrap();
    let path = directory.join("mutation-runner-0.replay");
    assert_eq!(
        result,
        Err(format!(
            "Seed {SEED}, run 0, expected {:?}: Injected check failed. Replay: {}",
            outcome(&input),
            path.display()
        ))
    );
    assert_eq!(read_replay(File::open(&path).unwrap()).unwrap(), input);
    let mut visits = 0;
    assert_eq!(
        replay(&directory, |restored| {
            visits += 1;
            assert_eq!(restored, &input);
            Err("Injected check failed.".to_owned())
        }),
        Err(format!(
            "{}: Expected {:?}: Injected check failed.",
            path.display(),
            outcome(&input)
        ))
    );
    assert_eq!(visits, 1);
    assert_eq!(replay(&directory, |_| Ok(())), Ok(1));
    fs::remove_dir_all(directory).unwrap();
}

#[test]
fn a_panicking_check_saves_a_replay_and_reports_the_panic_text() {
    let directory = scratch("a_panicking_check_saves_a_replay_and_reports_the_panic_text");
    let result = run_mutations(SEED, 1, &corpus(), &directory, |_| {
        panic!("Injected panic.")
    });
    let path = directory.join("mutation-runner-0.replay");
    let input = read_replay(File::open(&path).unwrap()).unwrap();
    assert_eq!(
        result,
        Err(format!(
            "Seed {SEED}, run 0, expected {:?}: Check panicked: Injected panic. Replay: {}",
            outcome(&input),
            path.display()
        ))
    );
    assert_eq!(replay(&directory, |_| Ok(())), Ok(1));
    fs::remove_dir_all(directory).unwrap();
}

#[test]
fn replay_checks_every_file_in_name_order_after_a_failure() {
    let directory = scratch("replay_checks_every_file_in_name_order_after_a_failure");
    let mut input = corpus().remove(0);
    input.width = 0;
    for name in ["b.replay", "a.replay"] {
        write_replay(&input, File::create(directory.join(name)).unwrap()).unwrap();
    }
    fs::write(directory.join(".gitkeep"), []).unwrap();
    let mut visits = 0;
    let result = replay(&directory, |_| {
        visits += 1;
        Err("Injected check failed.".to_owned())
    });
    assert_eq!(visits, 2);
    assert_eq!(
        result,
        Err(["a.replay", "b.replay"]
            .map(|name| format!(
                "{}: Expected {:?}: Injected check failed.",
                directory.join(name).display(),
                outcome(&input)
            ))
            .join("\n"))
    );
    fs::remove_dir_all(directory).unwrap();
}
