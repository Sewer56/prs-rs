use criterion::{black_box, BenchmarkId, Criterion, Throughput};
use prs_rs::comp::prs_compress_unsafe;
use prs_rs::util::prs_calculate_max_compressed_size;

use crate::util::{get_uncompressed_file_path, load_sample_file};

pub fn bench_compress_file(c: &mut Criterion) {
    let file_names = vec!["Model.bin" /* , "ObjectLayout.bin", "WorstCase.bin"*/];
    let mut group = c.benchmark_group("File Compression");

    for file_name in file_names {
        let original = load_sample_file(get_uncompressed_file_path(file_name));
        let mut comp_buf = vec![0_u8; prs_calculate_max_compressed_size(original.len())];

        group.throughput(Throughput::Bytes(original.len() as u64));
        group.bench_with_input(
            BenchmarkId::new("compress", file_name),
            &original,
            |b, original| {
                b.iter(|| {
                    let compressed_size = unsafe {
                        prs_compress_unsafe(
                            original.as_ptr(),
                            original.len(),
                            comp_buf.as_mut_slice(),
                        )
                    };
                    // Ensure the compiler does not optimize away the function's side-effects
                    black_box(compressed_size);
                })
            },
        );
    }

    group.finish();
}
