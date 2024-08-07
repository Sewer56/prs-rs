pub mod helpers;

use helpers::samples::{get_uncompressed_file_path, load_sample_file};
use prs_rs::comp::prs_compress_unsafe;
use prs_rs::decomp::prs_decompress_unsafe;
use prs_rs::util::prs_calculate_max_compressed_size;
use rstest::rstest;

#[rstest]
#[case::model("Model.bin")]
#[case::layout("ObjectLayout.bin")]
#[case::worstcase("WorstCase.bin")]
#[case::badending("BadEnding.bin")]
#[case::empty("Empty.bin")]
fn can_compress_and_decompress_file(#[case] file_name: &str) {
    let original = load_sample_file(get_uncompressed_file_path(file_name));
    let mut comp_buf = vec![0_u8; prs_calculate_max_compressed_size(original.len())];
    let compressed_size =
        unsafe { prs_compress_unsafe(original.as_ptr(), original.len(), comp_buf.as_mut_slice()) };

    // Adjust the buffer size to the actual compressed size
    comp_buf.resize(compressed_size, 0);

    // Decompress and verify
    let mut decomp_buf = vec![0_u8; original.len()];
    let decompressed_size =
        unsafe { prs_decompress_unsafe(comp_buf.as_slice(), decomp_buf.as_mut_slice()) };

    assert_eq!(original.len(), decompressed_size);

    // Compare original and decompressed data in blocks of 256 bytes
    // This approach allows for easier error diagnosis by identifying specific
    // blocks where decompression may have failed or produced incorrect results
    const CHUNK_SIZE: usize = 16;
    for (i, (orig_chunk, decomp_chunk)) in original
        .chunks(CHUNK_SIZE)
        .zip(decomp_buf.chunks(CHUNK_SIZE))
        .enumerate()
    {
        assert_eq!(
            orig_chunk,
            decomp_chunk,
            "Decompression output does not match the original file in block {} (bytes {} to {})",
            i,
            i * CHUNK_SIZE,
            i * CHUNK_SIZE + orig_chunk.len() - 1
        );
    }
}

#[rstest]
#[case::model("Model.bin")]
#[case::layout("ObjectLayout.bin")]
#[case::worstcase("WorstCase.bin")]
#[case::badending("BadEnding.bin")]
fn can_compress_and_decompress_file_with_nonzero_buffers(#[case] file_name: &str) {
    let original = load_sample_file(get_uncompressed_file_path(file_name));
    let mut comp_buf = vec![0xFF_u8; prs_calculate_max_compressed_size(original.len())];
    let compressed_size =
        unsafe { prs_compress_unsafe(original.as_ptr(), original.len(), comp_buf.as_mut_slice()) };

    // Adjust the buffer size to the actual compressed size
    comp_buf.resize(compressed_size, 0);

    // Decompress and verify
    let mut decomp_buf = vec![0xFF_u8; original.len()];
    let decompressed_size =
        unsafe { prs_decompress_unsafe(comp_buf.as_slice(), decomp_buf.as_mut_slice()) };

    assert_eq!(original.len(), decompressed_size);

    // Compare original and decompressed data in blocks of 256 bytes
    // This approach allows for easier error diagnosis by identifying specific
    // blocks where decompression may have failed or produced incorrect results
    const CHUNK_SIZE: usize = 16;
    for (i, (orig_chunk, decomp_chunk)) in original
        .chunks(CHUNK_SIZE)
        .zip(decomp_buf.chunks(CHUNK_SIZE))
        .enumerate()
    {
        assert_eq!(
            orig_chunk,
            decomp_chunk,
            "Decompression output does not match the original file in block {} (bytes {} to {})",
            i,
            i * CHUNK_SIZE,
            i * CHUNK_SIZE + orig_chunk.len() - 1
        );
    }
}
