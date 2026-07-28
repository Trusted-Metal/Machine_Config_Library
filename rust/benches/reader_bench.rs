// Phase 3.9 — Criterion benchmarks for the Rust reader.
// Run with:  cargo bench
// HTML reports land in target/criterion/

use criterion::{black_box, criterion_group, criterion_main, Criterion};
use machine_config::reader::MachineConfigReader;

static REFERENCE: &str = concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../fixtures/reference_config.h5"
);

/// End-to-end cost of open + parse (scalars only, no correction grids).
fn bench_parse(c: &mut Criterion) {
    c.bench_function("open_and_parse", |b| {
        b.iter(|| {
            MachineConfigReader::open(black_box(REFERENCE))
                .unwrap()
                .parse()
                .unwrap()
        })
    });
}

/// Cost of converting an already-opened reader to pretty-printed JSON.
fn bench_to_json(c: &mut Criterion) {
    let reader = MachineConfigReader::open(REFERENCE).unwrap();
    c.bench_function("to_json_pretty", |b| {
        b.iter(|| reader.to_json(black_box(true), black_box(false)).unwrap())
    });
}

/// Cost of open + parse_with_binary (correction grids included).
fn bench_parse_with_binary(c: &mut Criterion) {
    c.bench_function("open_and_parse_with_binary", |b| {
        b.iter(|| {
            MachineConfigReader::open(black_box(REFERENCE))
                .unwrap()
                .parse_with_binary()
                .unwrap()
        })
    });
}

criterion_group!(benches, bench_parse, bench_to_json, bench_parse_with_binary);
criterion_main!(benches);
