#![allow(dead_code, unused_imports)]

mod compress;
mod createcompdict;
mod decompress;
mod calc_decompressed_size;
mod util;

use compress::bench_compress_file;
use createcompdict::bench_create_dict;
use criterion::{criterion_group, criterion_main, Criterion};
use decompress::bench_decompress;
use calc_decompressed_size::bench_estimate;

#[cfg(not(target_os = "windows"))]
use pprof::criterion::{Output, PProfProfiler};

fn criterion_benchmark(c: &mut Criterion) {
    bench_estimate(c);
    bench_decompress(c);
    bench_compress_file(c);

    // Excluded from PGO.
    #[cfg(not(feature = "pgo"))]
    {
        bench_create_dict(c);
    }
}

#[cfg(not(target_os = "windows"))]
criterion_group! {
    name = benches;
    config = Criterion::default().with_profiler(PProfProfiler::new(100, Output::Flamegraph(None)));
    targets = criterion_benchmark
}

#[cfg(target_os = "windows")]
criterion_group! {
    name = benches;
    config = Criterion::default();
    targets = criterion_benchmark
}

criterion_main!(benches);
