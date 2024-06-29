use super::comp_dict::CompDict;
use core::mem::size_of;
use core::ptr::read_unaligned;

/// This trait specifies parameters for the [`lz77_get_longest_match`] function.
///
/// This allows for the compiler to generate different optimized versions of the function,
/// via the use of monomorphization and constant propagation.
pub trait Lz77Parameters {
    /// Maximum offset (from the current position) to search for a match.
    /// Specified as positive, so 0x1000 means 0x1000 bytes back.
    const MAX_OFFSET: usize;
    /// Maximum length of the match.
    const MAX_LENGTH: usize;
}

/// Searches back up to 'COPY_MAX_LENGTH' bytes and returns the length of the longest matching
/// sequence of bytes. This is the fast version that assumes there are more than 'COPY_MAX_LENGTH'
/// bytes left.
///
/// # Parameters
///
/// - `dict`: The dictionary used to speed up computation.
/// - `source_ptr`: The data where the match is to be searched.
/// - `source_len`: The length of the data.
/// - `source_index`: The index of the current byte in the source.
///
/// # Safety
///
/// Should be safe provided `dict` is initialized with `source` and composed of valid data.
#[inline(never)]
pub unsafe fn lz77_get_longest_match_fast<P: Lz77Parameters>(
    dict: &mut CompDict,
    source_ptr: *const u8,
    source_index: usize,
) -> Lz77Match {
    let mut best_match = Lz77Match {
        offset: 0,
        length: 0,
    };

    // Calculate the minimum offset to consider for a match
    let min_offset = source_index.saturating_sub(P::MAX_OFFSET);

    // Read the 2-byte sequence from source at the current index
    let key = read_unaligned(source_ptr.add(source_index) as *const u16);

    // Retrieve possible match offsets from the dictionary
    let offsets = dict.get_item(key, min_offset, source_index.saturating_sub(1));
    for &match_offset in offsets.iter().rev() {
        let match_offset = match_offset as usize;

        // Determine the length of the match
        let mut match_length = 2;

        // Check the next 2 bytes.
        let offset_src_ptr = source_ptr.add(match_offset);
        let offset_dst_ptr = source_ptr.add(source_index);
        let initial_match = read_unaligned(offset_src_ptr as *const u32)
            == read_unaligned(offset_dst_ptr as *const u32);

        if !initial_match {
            // Length is 2 or 3 bytes.
            match_length +=
                (*offset_src_ptr.add(match_length) == *offset_dst_ptr.add(match_length)) as usize;
        } else {
            match_length = 4;

            // We are usize aligned (for perf) and MAX_LENGTH should be divisible by usize.
            // Therefore there is no risk of running out of bounds here in the usize matching.
            debug_assert!(P::MAX_LENGTH % size_of::<usize>() == 0);

            // Check the next 4 bytes.
            // On 32-bit this redundant as it's part of the great LLVM unroll below.
            // But here we need to align.
            #[cfg(target_pointer_width = "64")]
            {
                // On 64-bit systems, ensure we're aligned to 8-byte boundary
                if match_length < P::MAX_LENGTH
                    && read_unaligned(offset_src_ptr.add(match_length) as *const u32)
                        == read_unaligned(offset_dst_ptr.add(match_length) as *const u32)
                {
                    match_length += 4;
                }
            }

            // First 8 bytes match.
            while match_length < P::MAX_LENGTH
                && read_unaligned(offset_src_ptr.add(match_length) as *const usize)
                    == read_unaligned(offset_dst_ptr.add(match_length) as *const usize)
            {
                match_length += size_of::<usize>();
            }

            // Cleverly unrolled by LLVM as 4 single byte checks.
            while match_length < P::MAX_LENGTH
                && *offset_src_ptr.add(match_length) == *offset_dst_ptr.add(match_length)
            {
                match_length += 1;
            }
        }

        // Update the best match if this match is longer
        if match_length > best_match.length {
            best_match.length = match_length;
            best_match.offset = match_offset as isize - source_index as isize;

            if match_length == P::MAX_LENGTH {
                break;
            }
        }
    }

    best_match
}

/// Searches back up to 'COPY_MAX_LENGTH' bytes and returns the length of the longest matching
/// sequence of bytes. This is the slow version that ensures we don't overrun past the end of file.
///
/// # Parameters
///
/// - `dict`: The dictionary used to speed up computation.
/// - `source_ptr`: The data where the match is to be searched.
/// - `source_len`: The length of the data.
/// - `source_index`: The index of the current byte in the source.
///
/// # Safety
///
/// Should be safe provided `dict` is initialized with `source` and composed of valid data.
#[inline(never)]
pub unsafe fn lz77_get_longest_match_slow<P: Lz77Parameters>(
    dict: &mut CompDict,
    source_ptr: *const u8,
    source_len: usize,
    source_index: usize,
) -> Lz77Match {
    let mut best_match = Lz77Match {
        offset: 0,
        length: 0,
    };

    // Calculate the minimum offset to consider for a match
    let min_offset = source_index.saturating_sub(P::MAX_OFFSET);

    // Read the 2-byte sequence from source at the current index
    let key = read_unaligned(source_ptr.add(source_index) as *const u16);

    // Calculate the maximum possible match length
    let max_match_length = P::MAX_LENGTH.min(source_len - source_index);

    // Retrieve possible match offsets from the dictionary
    let offsets = dict.get_item(key, min_offset, source_index.saturating_sub(1));
    for &match_offset in offsets.iter().rev() {
        let match_offset = match_offset as usize;

        // We start having matched 2 and match byte by byte
        let mut match_length = 2;
        let offset_src_ptr = source_ptr.add(match_offset);
        let offset_dst_ptr = source_ptr.add(source_index);
        while match_length < max_match_length
            && *offset_src_ptr.add(match_length) == *offset_dst_ptr.add(match_length)
        {
            match_length += 1;
        }

        // Update the best match if this match is longer
        if match_length > best_match.length {
            best_match.length = match_length;
            best_match.offset = match_offset as isize - source_index as isize;

            if match_length == max_match_length {
                break;
            }
        }
    }

    best_match
}

/// Represents a match in the LZ77 algorithm.
pub struct Lz77Match {
    /// Offset of the LZ77 match, expressed as a negative number.
    pub offset: isize,
    /// Length of the match.
    pub length: usize,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_longest_match_with_repetition() {
        let data = b"abcabcabcabcabc";
        let mut dict = CompDict::new(data.len());
        unsafe { dict.init(data, 0) }

        // Longest match for "abc" starting from index 3 should be of length 12
        let match_result = unsafe {
            lz77_get_longest_match_slow::<CompressParameters>(
                &mut dict,
                data.as_ptr(),
                data.len(),
                3,
            )
        };
        assert_eq!(match_result.length, 12);
        assert_eq!(match_result.offset, -3);
    }

    #[test]
    fn test_no_match() {
        let data = b"abcdefgh";
        let mut dict = CompDict::new(data.len());
        unsafe { dict.init(data, 0) }

        // No repetition, so no match
        let match_result = unsafe {
            lz77_get_longest_match_slow::<CompressParameters>(
                &mut dict,
                data.as_ptr(),
                data.len(),
                2,
            )
        };
        assert_eq!(match_result.length, 0);
    }

    #[test]
    fn test_multiple_matches() {
        let data = b"ababababab";
        let mut dict = CompDict::new(data.len());
        unsafe { dict.init(data, 0) }

        // Multiple "ab" patterns, longest match from index 2 should be length 8
        let match_result = unsafe {
            lz77_get_longest_match_slow::<CompressParameters>(
                &mut dict,
                data.as_ptr(),
                data.len(),
                2,
            )
        };
        assert_eq!(match_result.length, 8);
        assert_eq!(match_result.offset, -2);
    }

    #[test]
    fn test_boundary_conditions() {
        let data = b"ababababab";
        let mut dict = CompDict::new(data.len());
        unsafe { dict.init(data, 0) }

        // Testing boundary condition: match at the very end
        let match_result = unsafe {
            lz77_get_longest_match_slow::<CompressParameters>(
                &mut dict,
                data.as_ptr(),
                data.len(),
                data.len() - 3,
            )
        };
        assert_eq!(match_result.length, 3);
        assert_eq!(match_result.offset, -2);
    }

    #[test]
    fn test_last_match_on_boundary() {
        let data = b"acacacabab";
        let mut dict = CompDict::new(data.len());
        unsafe { dict.init(data, 0) }

        // Testing boundary condition: match at the very end, when very end is only pattern
        let match_result = unsafe {
            lz77_get_longest_match_slow::<CompressParameters>(
                &mut dict,
                data.as_ptr(),
                data.len(),
                data.len() - 2,
            )
        };
        assert_eq!(match_result.length, 2);
        assert_eq!(match_result.offset, -2);
    }

    struct CompressParameters;
    impl Lz77Parameters for CompressParameters {
        const MAX_OFFSET: usize = 0x1FFF;
        const MAX_LENGTH: usize = 256;
    }
}
