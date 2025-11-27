mod helpers;
use helpers::samples::{get_compressed_file_path, get_uncompressed_file_path, load_sample_file};
use more_asserts::assert_le;
use prs_rs::decomp::prs_calculate_decompressed_size;
use prs_rs::util::prs_calculate_max_compressed_size;
use rstest::rstest;

#[rstest]
#[case::model("Model.bin")]
#[case::layout("ObjectLayout.bin")]
#[case::worstcase("WorstCase.bin")]
fn can_estimate_file(#[case] file_name: &str) {
    let compressed = load_sample_file(get_compressed_file_path(file_name));
    let expected = load_sample_file(get_uncompressed_file_path(file_name));

    let estimated_size = unsafe { prs_calculate_decompressed_size(compressed.as_slice()) };
    assert_eq!(expected.len(), estimated_size)
}

#[rstest]
#[case::model("Model.bin")]
#[case::layout("ObjectLayout.bin")]
#[case::worstcase("WorstCase.bin")]
fn prs_calculate_max_compressed_size_is_sufficient(#[case] file_name: &str) {
    let compressed = load_sample_file(get_compressed_file_path(file_name));
    let uncompressed = load_sample_file(get_uncompressed_file_path(file_name));
    assert_le!(
        compressed.len(),
        prs_calculate_max_compressed_size(uncompressed.len())
    );
}
