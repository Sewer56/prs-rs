use crate::macros::AbortableResult;
use prs_rs::{comp::prs_compress_unsafe, util::prs_calculate_max_compressed_size};
use rayon::prelude::*;
use std::fs::{read, remove_file, write};
use std::path::Path;
use walkdir::WalkDir;

pub(crate) fn compress_files(path: &str) {
    let path = Path::new(path);
    if path.is_dir() {
        WalkDir::new(path)
            .into_iter()
            .filter_map(Result::ok)
            .filter(|e| e.file_type().is_file())
            .par_bridge()
            .for_each(|entry| {
                let input_path = entry.path();
                compress_file(
                    input_path,
                    format!("{}.prs", input_path.to_string_lossy()).as_ref(),
                );
            });
    } else {
        compress_file(path, format!("{}.prs", path.to_string_lossy()).as_ref());
    }
}

fn compress_file(input_path: &Path, output_path: &Path) {
    let original_data = read(input_path).unwrap_abort();
    let alloc_size = prs_calculate_max_compressed_size(original_data.len());
    let mut compressed_data = Box::<[u8]>::new_uninit_slice(alloc_size);
    let compressed_size = unsafe {
        prs_compress_unsafe(
            original_data.as_ptr(),
            original_data.len(),
            compressed_data.as_mut_ptr() as *mut u8,
        )
    };

    let compressed_data = unsafe { compressed_data.assume_init() };
    let compressed_slice = &compressed_data[0..compressed_size];
    write(output_path, compressed_slice).unwrap_abort();
    remove_file(input_path).unwrap_abort();
}
