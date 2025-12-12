use crate::macros::AbortableResult;
use prs_rs::decomp::{prs_calculate_decompressed_size, prs_decompress_unsafe};
use rayon::prelude::*;
use std::fs::{create_dir_all, read, remove_file, write};
use std::path::Path;
use walkdir::WalkDir;

pub(crate) fn decompress_files(source: &str, target: Option<&str>) {
    let source_path = Path::new(source);

    match target {
        Some(target_str) => {
            let target_path = Path::new(target_str);
            if source_path.is_dir() {
                decompress_directory_to_target(source_path, target_path);
            } else if source_path
                .extension()
                .is_some_and(|ext| ext.eq_ignore_ascii_case("prs"))
            {
                decompress_file_to_target(source_path, target_path);
            }
        }
        None => {
            // In-place mode: write next to source (without .prs) and delete original
            if source_path.is_dir() {
                WalkDir::new(source_path)
                    .into_iter()
                    .filter_map(Result::ok)
                    .filter(|e| {
                        e.file_type().is_file()
                            && e.path()
                                .extension()
                                .is_some_and(|ext| ext.eq_ignore_ascii_case("prs"))
                    })
                    .par_bridge()
                    .for_each(|entry| {
                        let input_path = entry.path();
                        let path_str = input_path.to_str().unwrap();
                        let output_path = &path_str[..path_str.len() - 4];
                        decompress_file_inplace(input_path, output_path.as_ref());
                    });
            } else if source_path
                .extension()
                .is_some_and(|ext| ext.eq_ignore_ascii_case("prs"))
            {
                let path_str = source_path.to_str().unwrap();
                let output_path = &path_str[..path_str.len() - 4];
                decompress_file_inplace(source_path, output_path.as_ref());
            }
        }
    }
}

fn decompress_directory_to_target(source_dir: &Path, target_dir: &Path) {
    WalkDir::new(source_dir)
        .into_iter()
        .filter_map(Result::ok)
        .filter(|e| {
            e.file_type().is_file()
                && e.path()
                    .extension()
                    .is_some_and(|ext| ext.eq_ignore_ascii_case("prs"))
        })
        .par_bridge()
        .for_each(|entry| {
            let input_path = entry.path();
            // Compute relative path from source directory
            let relative_path = input_path.strip_prefix(source_dir).unwrap();
            // Build output path: target_dir + relative_path (without .prs extension)
            let output_path = target_dir.join(relative_path);
            // Remove the .prs extension
            let output_str = output_path.to_string_lossy();
            let output_path: &Path = output_str[..output_str.len() - 4].as_ref();

            // Ensure parent directory exists
            if let Some(parent) = output_path.parent() {
                create_dir_all(parent).unwrap_abort();
            }

            decompress_file(input_path, output_path);
        });
}

fn decompress_file_to_target(source_file: &Path, target: &Path) {
    let output_path = if target.is_dir() {
        // Target is a directory, use source filename without .prs extension
        let filename = source_file.file_stem().unwrap();
        target.join(filename)
    } else {
        // Target is a file path, use as-is
        target.to_path_buf()
    };

    decompress_file(source_file, &output_path);
}

fn decompress_file(input_path: &Path, output_path: &Path) {
    let compressed_data = read(input_path).unwrap_abort();
    let estimate: usize = unsafe { prs_calculate_decompressed_size(compressed_data.as_ptr()) };
    let mut decomp = Box::<[u8]>::new_uninit_slice(estimate);

    unsafe { prs_decompress_unsafe(compressed_data.as_ptr(), decomp.as_mut_ptr() as *mut u8) };
    let decomp = unsafe { decomp.assume_init() };
    write(output_path, decomp).unwrap_abort();
}

fn decompress_file_inplace(input_path: &Path, output_path: &Path) {
    decompress_file(input_path, output_path);
    remove_file(input_path).unwrap_abort();
}
