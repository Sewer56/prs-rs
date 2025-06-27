use crate::util::{get_compressed_file_path, get_uncompressed_file_path, load_sample_file};
use criterion::{BenchmarkId, Criterion, Throughput};
use prs_rs::{
    comp::create_comp_dict,
    decomp::{prs_calculate_decompressed_size, prs_decompress_unsafe},
};
use std::hint::black_box;

pub fn bench_create_dict(c: &mut Criterion) {
    let file_names = vec!["Model.bin", "ObjectLayout.bin"];
    let mut group = c.benchmark_group("Compression Dictionary Creation");

    for file_name in file_names {
        let data = load_sample_file(get_uncompressed_file_path(file_name));
        group.throughput(Throughput::Bytes(data.len() as u64));
        group.bench_with_input(
            BenchmarkId::new("create_dict", file_name),
            &data,
            |b, data| b.iter(|| black_box(create_comp_dict(data))),
        );
    }

    group.finish();
}
