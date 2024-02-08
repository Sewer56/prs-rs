use std::{fs::{self, File}, io::prelude::Read, path::PathBuf};

use walkdir::WalkDir;

pub fn get_compressed_file_path(file_name: &str) -> PathBuf {
    let mut path = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    path.push("tests");
    path.push("samples");
    path.push("compressed");
    path.push(file_name.to_owned() + ".prs");
    path
}

pub fn get_uncompressed_file_path(file_name: &str) -> PathBuf {
    let mut path = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    path.push("tests");
    path.push("samples");
    path.push("uncompressed");
    path.push(file_name);
    path
}

pub fn get_sample_mod_files() -> Vec<PathBuf> {
    let mut path = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    path.push("tests");
    path.push("samples");
    path.push("mods");
    let str = path.into_os_string();

    let files: Vec<PathBuf> = WalkDir::new(str)
        .into_iter()
        .filter_map(|entry| entry.ok())
        .filter(|entry| entry.file_type().is_file())
        .map(|entry| entry.into_path())
        .collect();
    
    files
}


pub fn load_sample_file(path: PathBuf) -> Vec<u8> {
    let mut file = File::open(path).expect("Unable to open file");
    let mut buffer = Vec::new();
    file.read_to_end(&mut buffer).expect("Unable to read file");
    buffer
}
