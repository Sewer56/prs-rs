use crate::impls::comp::compress;
use crate::impls::decomp::estimate::prs_calculate_decompressed_size_impl;
use crate::prelude::Global;
use core::ffi::c_uchar;

/// Compresses the given data in `source`, placing it in `destimation`.
///
/// Parameters
///
/// - `src`: A pointer to the compressed data.
/// - `src_len`: Length of the compressed data.
/// - `destination`: A pointer to the decompressed data to be written.
///
/// # Returns
///
/// Number of bytes written to `destination`.
///
/// # Safety
///
/// It's safe as long as `dest` has sufficient length (max length: [`prs_calculate_max_compressed_size`])
/// and the remaining parameters are valid.
#[no_mangle]
#[inline(never)]
pub unsafe extern "C" fn prs_compress(
    src: *const c_uchar,
    dest: *mut c_uchar,
    src_len: usize,
) -> usize {
    compress::prs_compress::<Global, Global>(src, dest, src_len, Global, Global)
}

/// Decodes the maximum possible compressed size after compressing a file with provided
/// `source_len` length.
///
/// # Parameters
///
/// - `source_len`: Length of the compressed data.
///
/// # Returns
///
/// The length of the decompressed data at `source`.
///
/// # Remarks
///
/// A properly compressed PRS file has a theoretical maximum size of 1.125 times the size of the
/// original input. i.e. (1 byte for every 8 bytes of input).
///
/// Up to 2 bytes may be added to that in addition, namely via:
/// - Rounding file to next byte
/// - Having to write 00 opcode after a compressed sequence of bytes to terminate.
#[no_mangle]
#[inline(never)]
pub extern "C" fn prs_calculate_max_compressed_size(source_len: usize) -> usize {
    crate::util::prs_calculate_max_compressed_size(source_len)
}

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
#[no_mangle]
#[inline(never)]
pub unsafe extern "C" fn prs_calculate_decompressed_size(src: *const c_uchar) -> usize {
    prs_calculate_decompressed_size_impl(src)
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
/// call [`prs_calculate_max_compressed_size`] to get the length of the decompressed data
/// buffer.
///
/// If you are unsure of the length, you use the [`prs_calculate_decompressed_size`]
/// function to determine the length of the decompressed data (at expense of some additional overhead).
///
/// # Safety
///
/// Function is safe as long as the source points to valid PRS compressed data with
/// a terminator byte. The destination should be large enough to store the decompressed data.
#[no_mangle]
#[inline(never)]
pub unsafe extern "C" fn prs_decompress(src: *const c_uchar, dest: *mut c_uchar) -> usize {
    crate::decomp::prs_decompress_unsafe(src, dest)
}
