use crate::impls::comp::compress::COPY_MAX_LENGTH;

use super::comp_dict::CompDict;
use super::compress::MAX_OFFSET;
use core::mem::size_of;
use core::ptr::read_unaligned;

/// Searches back up to 'COPY_MAX_LENGTH' bytes and returns the length of the longest matching
/// sequence of bytes. This is the fast version that assumes there are enough bytes left.
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
#[inline(always)]
pub unsafe fn lz77_get_longest_match_fast(
    dict: &mut CompDict,
    source_ptr: *const u8,
    source_index: usize,
) -> Lz77Match {
    let mut best_match = Lz77Match {
        offset: 0,
        length: 0,
    };

    // Calculate the minimum offset to consider for a match
    let min_offset = source_index.saturating_sub(MAX_OFFSET);

    // Read the 2-byte sequence from source at the current index
    let key = read_unaligned(source_ptr.add(source_index) as *const u16);

    // Retrieve possible match offsets from the dictionary
    let offsets = dict.get_item(key, min_offset, source_index.saturating_sub(1));
    for &match_offset in offsets.iter().rev() {
        let match_offset = match_offset as usize;

        // Determine the length of the match
        let mut match_length = 0;

        // Check the next 6 bytes.
        // We reset to offset 0 because COPY_MAX_LENGTH divides into it, allowing
        // for faster matching with completely repeated sequences
        debug_assert!(COPY_MAX_LENGTH as usize % size_of::<usize>() == 0);
        let offset_src_ptr = source_ptr.add(match_offset);
        let offset_dst_ptr = source_ptr.add(source_index);
        let initial_match = read_unaligned(offset_src_ptr.add(match_length) as *const usize)
            == read_unaligned(offset_dst_ptr.add(match_length) as *const usize);

        if !initial_match {
            match_length = 2;

            // Length is < 8 bytes. So we don't need to compare COPY_MAX_LENGTH.

            // Note: This code is normally inefficient but LLVM optimizes it down to bitshifts
            // nicely. Normally I'd do this by hand but letting LLVM do it means we get decent
            // codegen for 32-bit too.
            if *offset_src_ptr.add(match_length) == *offset_dst_ptr.add(match_length) {
                match_length += 1;

                // 32-bit can match +1 only (up to 3)
                // 64-bit can match +5, up to 7 ()
                #[cfg(target_pointer_width = "64")]
                {
                    if *offset_src_ptr.add(match_length) == *offset_dst_ptr.add(match_length) {
                        match_length += 1;
                        if *offset_src_ptr.add(match_length) == *offset_dst_ptr.add(match_length) {
                            match_length += 1;
                            if *offset_src_ptr.add(match_length)
                                == *offset_dst_ptr.add(match_length)
                            {
                                match_length += 1;
                                if *offset_src_ptr.add(match_length)
                                    == *offset_dst_ptr.add(match_length)
                                {
                                    match_length += 1;
                                }
                            }
                        }
                    }
                }
            }
        } else {
            // First 8 bytes match.
            while match_length < COPY_MAX_LENGTH as usize
                && read_unaligned(offset_src_ptr.add(match_length) as *const usize)
                    == read_unaligned(offset_dst_ptr.add(match_length) as *const usize)
            {
                match_length += size_of::<usize>();
            }

            // Cleverly unrolled by LLVM as 4 single byte checks.
            while match_length < COPY_MAX_LENGTH as usize
                && *offset_src_ptr.add(match_length) == *offset_dst_ptr.add(match_length)
            {
                match_length += 1;
            }
        }

        // Update the best match if this match is longer
        if match_length > best_match.length {
            best_match.length = match_length;
            best_match.offset = match_offset as isize - source_index as isize;

            if match_length == COPY_MAX_LENGTH as usize {
                break;
            }
        }
    }

    best_match
}

/// Searches back up to 'COPY_MAX_LENGTH' bytes and returns the length of the longest matching
/// sequence of bytes. This is the slow version that checks for bounds.
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
#[inline(always)]
pub unsafe fn lz77_get_longest_match_slow(
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
    let min_offset = source_index.saturating_sub(MAX_OFFSET);

    // Read the 2-byte sequence from source at the current index
    let key = read_unaligned(source_ptr.add(source_index) as *const u16);

    // Retrieve possible match offsets from the dictionary
    let offsets = dict.get_item(key, min_offset, source_index.saturating_sub(1));
    for &match_offset in offsets.iter().rev() {
        let match_offset = match_offset as usize;

        // We start having matched 2 and match byte by byte
        let mut match_length = 2;
        let offset_src_ptr = source_ptr.add(match_offset);
        let offset_dst_ptr = source_ptr.add(source_index);
        while match_length < COPY_MAX_LENGTH as usize
            && source_index + match_length < source_len
            && *offset_src_ptr.add(match_length) == *offset_dst_ptr.add(match_length)
        {
            match_length += 1;
        }

        // Update the best match if this match is longer
        if match_length > best_match.length {
            best_match.length = match_length;
            best_match.offset = match_offset as isize - source_index as isize;

            if match_length == COPY_MAX_LENGTH as usize {
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
        let match_result =
            unsafe { lz77_get_longest_match_slow(&mut dict, data.as_ptr(), data.len(), 3) };
        assert_eq!(match_result.length, 12);
        assert_eq!(match_result.offset, -3);
    }

    #[test]
    fn test_no_match() {
        let data = b"abcdefgh";
        let mut dict = CompDict::new(data.len());
        unsafe { dict.init(data, 0) }

        // No repetition, so no match
        let match_result =
            unsafe { lz77_get_longest_match_slow(&mut dict, data.as_ptr(), data.len(), 2) };
        assert_eq!(match_result.length, 0);
    }

    #[test]
    fn test_multiple_matches() {
        let data = b"ababababab";
        let mut dict = CompDict::new(data.len());
        unsafe { dict.init(data, 0) }

        // Multiple "ab" patterns, longest match from index 2 should be length 8
        let match_result =
            unsafe { lz77_get_longest_match_slow(&mut dict, data.as_ptr(), data.len(), 2) };
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
            lz77_get_longest_match_slow(&mut dict, data.as_ptr(), data.len(), data.len() - 3)
        };
        assert_eq!(match_result.length, 3);
        assert_eq!(match_result.offset, -2);

        // Testing boundary condition: no match beyond data length
        // Uncommented due to out of bounds access not present in actual workloads.
        /*
        let match_result = unsafe {
            lz77_get_longest_match(
                &mut dict,
                data.as_ptr(),
                data.len(),
                data.len(),
                15,
                15,
                false,
            )
        };
        assert_eq!(match_result.length, 0);
        */
    }

    #[test]
    fn test_last_match_on_boundary() {
        let data = b"acacacabab";
        let mut dict = CompDict::new(data.len());
        unsafe { dict.init(data, 0) }

        // Testing boundary condition: match at the very end, when very end is only pattern
        let match_result = unsafe {
            lz77_get_longest_match_slow(&mut dict, data.as_ptr(), data.len(), data.len() - 2)
        };
        assert_eq!(match_result.length, 2);
        assert_eq!(match_result.offset, -2);
    }
}
