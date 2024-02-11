use crate::macros::AbortableResult;
use prs_rs::decomp::{prs_calculate_decompressed_size, prs_decompress_unsafe};
use rayon::prelude::*;
use std::fs::{read, remove_file, write};
use std::path::Path;
use walkdir::WalkDir;

pub(crate) fn decompress_files(path: &str) {
    let path = Path::new(path);
    if path.is_dir() {
        WalkDir::new(path)
            .into_iter()
            .filter_map(Result::ok)
            .filter(|e| {
                e.file_type().is_file()
                    && e.path()
                        .extension()
                        .map_or(false, |ext| ext.eq_ignore_ascii_case("prs"))
            })
            .par_bridge()
            .for_each(|entry| {
                let input_path = entry.path();
                let path_str = input_path.to_str().unwrap();
                let output_path = &path_str[..path_str.len() - 4];
                decompress_file(input_path, output_path.as_ref());
            });
    } else if path
        .extension()
        .map_or(false, |ext| ext.eq_ignore_ascii_case("prs"))
    {
        let path_str = path.to_str().unwrap();
        let output_path = &path_str[..path_str.len() - 4];
        decompress_file(path, output_path.as_ref());
    }
}

fn decompress_file(input_path: &Path, output_path: &Path) {
    let compressed_data = read(input_path).unwrap_abort();
    let estimate: usize = unsafe { prs_calculate_decompressed_size(compressed_data.as_ptr()) };
    let mut decomp = Box::<[u8]>::new_uninit_slice(estimate);

    unsafe { prs_decompress_unsafe(compressed_data.as_ptr(), decomp.as_mut_ptr() as *mut u8) };
    let decomp = unsafe { decomp.assume_init() };
    write(output_path, decomp).unwrap_abort();
    remove_file(input_path).unwrap_abort();
}
