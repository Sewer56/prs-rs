use crate::macros::AbortableResult;
use prs_rs::{
    comp::prs_compress_unsafe,
    decomp::{prs_calculate_decompressed_size, prs_decompress_unsafe},
    util::prs_calculate_max_compressed_size,
};
use rayon::prelude::*;
use std::fs::read;
use std::path::Path;
use walkdir::WalkDir;

pub(crate) fn test_compression_mt(path: &str) {
    let path = Path::new(path);
    if path.is_dir() {
        WalkDir::new(path)
            .into_iter()
            .filter_map(|e| e.ok())
            .filter(|e| e.path().is_file())
            .par_bridge()
            .for_each(|entry| {
                test_file(entry.path());
            });
    } else {
        abort!("The path does not exist, is not a folder, or is not accessible.");
    }
}

pub(crate) fn test_compression(path: &str) {
    let path = Path::new(path);
    if path.is_dir() {
        for entry in WalkDir::new(path)
            .into_iter()
            .filter_map(|e| e.ok())
            .filter(|e| e.path().is_file())
        {
            test_file(entry.path());
        }
    } else if path.is_file() {
        test_file(path);
    } else {
        abort!("The path does not exist or is not accessible.");
    }
}

fn test_file(file_path: &Path) {
    println!("TEST: {}", file_path.display());
    let original_data = read(file_path).unwrap_abort();

    // Placeholder for compression and decompression logic
    if original_data.is_empty() {
        return;
    }

    let mut comp =
        Box::<[u8]>::new_uninit_slice(prs_calculate_max_compressed_size(original_data.len()));
    unsafe {
        prs_compress_unsafe(
            original_data.as_ptr(),
            original_data.len(),
            comp.as_mut_ptr() as *mut u8,
        )
    };

    let comp = unsafe { comp.assume_init() };
    let estimate: usize = unsafe { prs_calculate_decompressed_size(&*comp) };

    let mut decomp = Box::<[u8]>::new_uninit_slice(estimate);
    let decompressed_data =
        unsafe { prs_decompress_unsafe(&*comp, decomp.as_mut_ptr() as *mut u8) };
    let decomp = unsafe { decomp.assume_init() };

    // Compare original and decompressed data
    if estimate != decompressed_data {
        abort!(
            "Fail: {}. Decompressed len doesn't match.",
            file_path.display()
        );
    }

    if original_data.as_slice() != &*decomp {
        abort!(
            "Fail: {}. Decompressed len doesn't match.",
            file_path.display()
        );
    }
}
