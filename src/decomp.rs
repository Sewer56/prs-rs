use crate::{
    impls::decomp::{decompress::prs_decompress, estimate::prs_calculate_decompressed_size_impl},
    MutablePointerSrc, ReadOnlyPointerSrc,
};

/// Decodes the compressed data at `source` without performing the actual decompression.
///
/// You can use this operation to determine the size of the data to decompress
/// without actually decompressing the data to a buffer.
///
/// # Parameters
///
/// - `source`: A pointer to the compressed data.
///
/// # Returns
///
/// The length of the decompressed data at `source`.
///
/// # Safety
///
/// Function is safe as long as the pointer points to valid PRS compressed data with
/// a terminator byte.
pub unsafe fn prs_calculate_decompressed_size<S: ReadOnlyPointerSrc>(src: S) -> usize {
    prs_calculate_decompressed_size_impl(src.as_ptr())
}

/// Decompresses PRS compressed data, in an unsafe manner, without any error handling.
///
/// # Parameters
///
/// - `source`: A pointer to the compressed data.
/// - `destination`: A pointer to the decompressed data.
///
/// # Returns
///
/// - The length of the decompressed data.
///
/// # Remarks
///
/// The length of the decompressed data at `destination` should be sufficient to store the decompressed data.
///
/// If you know the length of the compressed data (i.e. amount of bytes until end of compressed data),
/// call [`crate::util::prs_calculate_max_decompressed_size`] to get the length of the decompressed data
/// buffer.
///
/// If you are unsure of the length, you use the [`crate::estimate::prs_calculate_decompressed_size`]
/// function to determine the length of the decompressed data (at expense of some additional overhead).
///
/// # Safety
///
/// Function is safe as long as the source points to valid PRS compressed data with
/// a terminator byte. The destination should be large enough to store the decompressed data.
pub unsafe fn prs_decompress_unsafe<S: ReadOnlyPointerSrc, T: MutablePointerSrc>(
    src: S,
    mut dest: T,
) -> usize {
    prs_decompress(src.as_ptr(), dest.as_mut_ptr())
}
