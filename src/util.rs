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
pub fn prs_calculate_max_compressed_size(source_len: usize) -> usize {
    ((source_len * 9) / 8) + 3 // +1 for integer division error
}
