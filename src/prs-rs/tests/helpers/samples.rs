use std::{fs::File, io::prelude::Read, path::PathBuf};

#[allow(dead_code)]
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

pub fn load_sample_file(path: PathBuf) -> Vec<u8> {
    let mut file = File::open(path).expect("Unable to open file");
    let mut buffer = Vec::new();
    file.read_to_end(&mut buffer).expect("Unable to read file");
    buffer
}
