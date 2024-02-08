use prs_rs::comp::prs_compress_unsafe;
use prs_rs::util::prs_calculate_max_compressed_size;
use prs_rs::decomp::{prs_calculate_decompressed_size, prs_decompress_unsafe};
use crate::util::{get_sample_mod_files, load_sample_file};

pub fn generate_pgo_data() {
    let files = get_sample_mod_files();
    for file in files {
        let data = load_sample_file(file.clone());

        // Compress it.
        let mut comp_buf = vec![0_u8; prs_calculate_max_compressed_size(data.len())];
        unsafe {
            prs_compress_unsafe(data.as_ptr(), data.len(), comp_buf.as_mut_slice());
        }

        // Estimate & Decompress it
        let decompressed_len = unsafe { prs_calculate_decompressed_size(comp_buf.as_slice()) };
        let mut decompressed = vec![0_u8; decompressed_len];
        unsafe {
            prs_decompress_unsafe(comp_buf.as_slice(), decompressed.as_mut_slice());
        }
        println!("Processed: {:?}", file);
    }
}