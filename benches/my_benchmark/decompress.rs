use crate::util::{get_compressed_file_path, load_sample_file};
use criterion::{Criterion, Throughput};
use prs_rs::decomp::{prs_calculate_decompressed_size, prs_decompress_unsafe};

pub fn bench_decompress(c: &mut Criterion) {
    let file_names = vec!["Model.bin", "ObjectLayout.bin", "WorstCase.bin"];
    let mut group = c.benchmark_group("File Decompression");

    for file_name in file_names {
        let compressed = load_sample_file(get_compressed_file_path(file_name));
        let decompressed_len = unsafe { prs_calculate_decompressed_size(compressed.as_slice()) };
        let mut decompressed = vec![0_u8; decompressed_len];
        group.throughput(Throughput::Bytes(decompressed_len as u64));
        group.bench_function(&format!("can_decompress_file_{}", file_name), |b| {
            b.iter(|| unsafe {
                prs_decompress_unsafe(compressed.as_slice(), decompressed.as_mut_slice())
            })
        });
    }
    
    group.finish();
}
