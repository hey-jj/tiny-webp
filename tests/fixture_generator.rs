//! The generator gives the same bytes on every run.

#[path = "../fixtures/generator.rs"]
mod generator;

#[test]
fn two_runs_of_the_generator_agree_byte_for_byte_on_every_fixture() {
    let first = generator::all();
    let second = generator::all();
    assert_eq!(first.len(), second.len());
    for (left, right) in first.iter().zip(second.iter()) {
        assert_eq!(
            (left.name, left.width, left.height),
            (right.name, right.width, right.height)
        );
        assert_eq!(left.rgba, right.rgba, "{} moved between runs", left.name);
    }
}

#[test]
fn every_fixture_holds_four_bytes_for_each_of_its_pixels() {
    for fixture in generator::all() {
        assert_eq!(
            fixture.rgba.len(),
            fixture.width as usize * fixture.height as usize * 4,
            "{}",
            fixture.name
        );
    }
}
