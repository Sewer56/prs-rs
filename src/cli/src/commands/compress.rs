use crate::macros::AbortableResult;
use prs_rs::{comp::prs_compress_unsafe, util::prs_calculate_max_compressed_size};
use rayon::prelude::*;
use std::fs::{create_dir_all, read, remove_file, write};
use std::path::Path;
use walkdir::WalkDir;

pub(crate) fn compress_files(source: &str, target: Option<&str>) {
    let source_path = Path::new(source);

    match target {
        Some(target_str) => {
            let target_path = Path::new(target_str);
            if source_path.is_dir() {
                compress_directory_to_target(source_path, target_path);
            } else {
                compress_file_to_target(source_path, target_path);
            }
        }
        None => {
            // In-place mode: write next to source and delete original
            if source_path.is_dir() {
                WalkDir::new(source_path)
                    .into_iter()
                    .filter_map(Result::ok)
                    .filter(|e| e.file_type().is_file())
                    .par_bridge()
                    .for_each(|entry| {
                        let input_path = entry.path();
                        let output_path = format!("{}.prs", input_path.to_string_lossy());
                        compress_file_inplace(input_path, output_path.as_ref());
                    });
            } else {
                let output_path = format!("{}.prs", source_path.to_string_lossy());
                compress_file_inplace(source_path, output_path.as_ref());
            }
        }
    }
}

fn compress_directory_to_target(source_dir: &Path, target_dir: &Path) {
    WalkDir::new(source_dir)
        .into_iter()
        .filter_map(Result::ok)
        .filter(|e| e.file_type().is_file())
        .par_bridge()
        .for_each(|entry| {
            let input_path = entry.path();
            // Compute relative path from source directory
            let relative_path = input_path.strip_prefix(source_dir).unwrap();
            // Build output path: target_dir + relative_path + .prs
            let mut output_path = target_dir.join(relative_path);
            let new_filename = format!(
                "{}.prs",
                output_path.file_name().unwrap().to_string_lossy()
            );
            output_path.set_file_name(new_filename);

            // Ensure parent directory exists for recursive structures
            if let Some(parent) = output_path.parent() {
                create_dir_all(parent).unwrap_abort();
            }

            compress_file(input_path, &output_path);
        });
}

fn compress_file_to_target(source_file: &Path, target: &Path) {
    let output_path = if target.is_dir() {
        // Target is a directory, use source filename with .prs extension
        let filename = format!(
            "{}.prs",
            source_file.file_name().unwrap().to_string_lossy()
        );
        target.join(filename)
    } else {
        // Target is a file path, use as-is
        target.to_path_buf()
    };

    compress_file(source_file, &output_path);
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
}

fn compress_file_inplace(input_path: &Path, output_path: &Path) {
    compress_file(input_path, output_path);
    remove_file(input_path).unwrap_abort();
}
