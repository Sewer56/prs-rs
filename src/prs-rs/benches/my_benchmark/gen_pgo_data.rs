use crate::util::{get_sample_mod_files, load_sample_file};
#[cfg(feature = "pgo")]
use prs_rs::exports::{prs_calculate_decompressed_size, prs_compress, prs_decompress};
use prs_rs::util::prs_calculate_max_compressed_size;

#[cfg(feature = "pgo")]
pub fn generate_pgo_data() {
    let files = get_sample_mod_files();
    for file in files {
        let data = load_sample_file(file.clone());

        // Compress it.
        let mut comp_buf = vec![0_u8; prs_calculate_max_compressed_size(data.len())];
        let compressed_size =
            unsafe { prs_compress(data.as_ptr(), comp_buf.as_mut_ptr(), data.len()) };
        comp_buf.truncate(compressed_size);

        // Estimate & Decompress it
        let decompressed_len = unsafe { prs_calculate_decompressed_size(comp_buf.as_ptr()) };
        let mut decompressed = vec![0_u8; decompressed_len];
        unsafe {
            prs_decompress(comp_buf.as_ptr(), decompressed.as_mut_ptr());
        }
        println!("Processed: {file:?}");
    }
}
