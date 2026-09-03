//! Times `photo-large` at quality 75.

#[path = "../fixtures/generator.rs"]
mod generator;

use std::hint::black_box;

use criterion::{criterion_group, criterion_main, Criterion};
use tiny_webp::Options;

fn encode_one_fixture(c: &mut Criterion) {
    let fixture = generator::all()
        .into_iter()
        .find(|candidate| candidate.name == "photo-large")
        .expect("the photo-large fixture is in the table");
    let mut opts = Options::default();
    opts.quality = 75;

    c.bench_function("photo-large q75", |b| {
        b.iter(|| {
            tiny_webp::encode_rgba(
                black_box(&fixture.rgba),
                fixture.width,
                fixture.height,
                &opts,
            )
        });
    });
}

criterion_group!(benches, encode_one_fixture);
criterion_main!(benches);
