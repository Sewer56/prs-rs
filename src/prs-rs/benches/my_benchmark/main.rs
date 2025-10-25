#![allow(dead_code, unused_imports)]

mod calc_decompressed_size;
mod compress;
mod createcompdict;
mod decompress;
mod gen_pgo_data;
mod util;

use calc_decompressed_size::bench_estimate;
use compress::bench_compress_file;
use createcompdict::bench_create_dict;
use criterion::{criterion_group, criterion_main, Criterion};
use decompress::bench_decompress;
#[cfg(feature = "pgo")]
use gen_pgo_data::generate_pgo_data;

#[allow(unused_variables)]
fn criterion_benchmark(c: &mut Criterion) {
    // Excluded from PGO.
    #[cfg(not(feature = "pgo"))]
    {
        bench_estimate(c);
        bench_decompress(c);
        bench_compress_file(c);
        bench_create_dict(c);
    }

    // Excluded from PGO.
    #[cfg(feature = "pgo")]
    {
        generate_pgo_data();
    }
}

criterion_group! {
    name = benches;
    config = Criterion::default();
    targets = criterion_benchmark
}

criterion_main!(benches);
