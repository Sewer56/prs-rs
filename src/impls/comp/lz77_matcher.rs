use core::ptr::read_unaligned;

use super::comp_dict::CompDict;

/// Searches back up to 'max_length' bytes and returns the length of the longest matching
/// sequence of bytes.
///
/// # Parameters
///
/// - `dict`: The dictionary used to speed up computation.
/// - `source_ptr`: The data where the match is to be searched.
/// - `source_len`: The length of the data.
/// - `source_index`: The index of the current byte in the source.
/// - `max_offset`: The maximum offset to search backwards. (constant, optimized away by LLVM)
/// - `max_length`: The maximum length to search backwards. (constant, optimized away by LLVM)
///
/// # Safety
///
/// Should be safe provided `dict` is initialized with `source` and composed of valid data.
#[inline]
pub unsafe fn lz77_get_longest_match(
    dict: &mut CompDict,
    source_ptr: *const u8,
    source_len: usize,
    source_index: usize,
    max_offset: usize,
    max_length: usize,
) -> Lz77Match {
    let mut best_match = Lz77Match {
        offset: 0,
        length: 0,
    };

    // Calculate the minimum offset to consider for a match
    let min_offset = if source_index > max_offset {
        source_index - max_offset
    } else {
        0
    };

    // Read the 2-byte sequence from source at the current index
    let key = read_unaligned(source_ptr.add(source_index) as *const u16);

    // Retrieve possible match offsets from the dictionary
    let offsets = dict.get_item(key, min_offset, source_index.saturating_sub(1));
    for &match_offset in offsets.iter().rev() {
        // I swear Rust is magical, reverse iteration has no overhead here (checked ASM)
        let match_offset = match_offset as usize;

        // Determine the length of the match
        let mut match_length = 2;
        while match_length < max_length
            && source_index + match_length < source_len
            && *source_ptr.add(match_offset + match_length)
                == *source_ptr.add(source_index + match_length)
        {
            match_length += 1;
        }

        // Update the best match if this match is longer
        if match_length > best_match.length {
            best_match.length = match_length;
            best_match.offset = match_offset as isize - source_index as isize;
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
        let mut dict = unsafe { CompDict::new(data) };

        // Longest match for "abc" starting from index 3 should be of length 12
        let match_result =
            unsafe { lz77_get_longest_match(&mut dict, data.as_ptr(), data.len(), 3, 15, 15) };
        assert_eq!(match_result.length, 12);
        assert_eq!(match_result.offset, -3);
    }

    #[test]
    fn test_no_match() {
        let data = b"abcdefgh";
        let mut dict = unsafe { CompDict::new(data) };

        // No repetition, so no match
        let match_result =
            unsafe { lz77_get_longest_match(&mut dict, data.as_ptr(), data.len(), 2, 15, 15) };
        assert_eq!(match_result.length, 0);
    }

    #[test]
    fn test_multiple_matches() {
        let data = b"ababababab";
        let mut dict = unsafe { CompDict::new(data) };

        // Multiple "ab" patterns, longest match from index 2 should be length 8
        let match_result =
            unsafe { lz77_get_longest_match(&mut dict, data.as_ptr(), data.len(), 2, 15, 15) };
        assert_eq!(match_result.length, 8);
        assert_eq!(match_result.offset, -2);
    }

    #[test]
    fn test_boundary_conditions() {
        let data = b"ababababab";
        let mut dict = unsafe { CompDict::new(data) };

        // Testing boundary condition: match at the very end
        let match_result = unsafe {
            lz77_get_longest_match(&mut dict, data.as_ptr(), data.len(), data.len() - 2, 15, 15)
        };
        assert_eq!(match_result.length, 2);
        assert_eq!(match_result.offset, -2);

        // Testing boundary condition: no match beyond data length
        let match_result = unsafe {
            lz77_get_longest_match(&mut dict, data.as_ptr(), data.len(), data.len(), 15, 15)
        };
        assert_eq!(match_result.length, 0);
    }

    #[test]
    fn test_last_match_on_boundary() {
        let data = b"acacacabab";
        let mut dict = unsafe { CompDict::new(data) };

        // Testing boundary condition: match at the very end, when very end is only pattern
        let match_result = unsafe {
            lz77_get_longest_match(&mut dict, data.as_ptr(), data.len(), data.len() - 2, 15, 15)
        };
        assert_eq!(match_result.length, 2);
        assert_eq!(match_result.offset, -2);
    }
}
