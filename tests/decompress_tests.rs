pub mod helpers;

use helpers::samples::get_compressed_file_path;
use helpers::samples::get_uncompressed_file_path;
use helpers::samples::load_sample_file;
use prs_rs::decomp::prs_decompress_unsafe;
use rstest::rstest;

#[rstest]
#[case::model("Model.bin")]
#[case::layout("ObjectLayout.bin")]
#[case::worstcase("WorstCase.bin")]
fn can_decompress_file(#[case] file_name: &str) {
    let compressed = load_sample_file(get_compressed_file_path(file_name));
    let expected = load_sample_file(get_uncompressed_file_path(file_name));

    let mut decomp_buf = vec![0_u8; expected.len()];
    let decompressed_size =
        unsafe { prs_decompress_unsafe(compressed.as_slice(), decomp_buf.as_mut_slice()) };
    assert_eq!(expected.len(), decompressed_size);
    assert_eq!(expected.as_slice(), decomp_buf.as_slice());
}
