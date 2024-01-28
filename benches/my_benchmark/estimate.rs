use criterion::Criterion;
use prs_rs::decomp::prs_calculate_decompressed_size;

use crate::util::{get_compressed_file_path, load_sample_file};
pub fn bench_estimate(c: &mut Criterion) {
    let file_names = vec!["Model.bin", "ObjectLayout.bin", "WorstCase.bin"];

    for file_name in file_names {
        let compressed = load_sample_file(get_compressed_file_path(file_name));
        c.bench_function(&format!("can_estimate_file_{}", file_name), |b| {
            b.iter(|| unsafe { prs_calculate_decompressed_size(compressed.as_slice()) })
        });
    }
}
