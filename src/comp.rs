use crate::{
    impls::comp::{comp_dict::CompDict, compress::prs_compress},
    MutablePointerSrc,
};

/// BENCHMARK ONLY, DO NOT USE
#[doc(hidden)]
pub fn create_comp_dict(data: &[u8]) -> u32 {
    unsafe {
        let mut dict = CompDict::new(data.len());
        dict.init(data, 0);
        dict.get_item(0, 0, u32::MAX as usize)[0]
    }
}

/// Compresses the given data in `source`, placing it in `destimation`.
///
/// Parameters
///
/// - `src`: A pointer to the decompressed data.
/// - `src_len`: Length of the decompressed data.
/// - `destination`: A pointer to the compressed data to be written.
///
/// # Returns
///
/// Number of bytes written to `destination`.
///
/// # Safety
///
/// It's safe as long as `dest` has sufficient length (max length: [`crate::util::prs_calculate_max_decompressed_size`])
/// and the remaining parameters are valid.
pub unsafe fn prs_compress_unsafe<T: MutablePointerSrc>(
    src: *const u8,
    src_len: usize,
    mut dest: T,
) -> usize {
    prs_compress(src, dest.as_mut_ptr(), src_len)
}
