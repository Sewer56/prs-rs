use super::lz77_matcher::{
    lz77_get_longest_match_fast, lz77_get_longest_match_slow, Lz77Match, Lz77Parameters,
};
use crate::impls::comp::comp_dict::CompDict;
use crate::prelude::Allocator;
use core::{ptr::write_unaligned, slice};

/// Size of a CompDict window.
///
/// This is the size for the look behind buffer (sized MAX_OFFSET) and the following lookahead buffer
/// (sized WINDOW_SIZE - MAX_OFFSET)
///
/// We process the data in smaller windows, to reduce RAM usage and improve L2 cache hit rate on modern CPUs.
/// This must be at least `MAX_OFFSET + COPY_MAX_LENGTH`.
///
/// This should be set based on available amount of L2 cache.
///
/// ----------------
///
/// As per CompDict Layout
/// - [CompDictEntry; MAX_U16] (dict), constant size.
/// - [MaxOffset; WINDOW_SIZE] (offsets), variable size. This buffer stores offsets of all items of 2 byte combinations.
///
/// Which is:
/// - 12/24 (CompDictEntry) * 64K = 768K/1.5M
/// - 4 (MaxOffset) * WINDOW_SIZE = 4 * 64K = 256K
///
/// During init we also use up:
/// - 4/8 (InsertPointer) * 64K = 256K/512K
/// - 2 (FreqTableEntry) * 64K = 128K
const WINDOW_SIZE: usize = u16::MAX as usize;

const MAX_OFFSET: usize = 0x1FFF;
const COPY_MAX_LENGTH: isize = 0x100;
const SHORT_COPY_MAX_OFFSET: isize = 0x100;
const SHORT_COPY_MAX_LEN: usize = 5;
const SHORT_COPY_MIN_LEN: usize = 2;

/// Parameters
///
/// - `source`: A pointer to the decompressed data.
/// - `destination`: A pointer to where to put the compressed data.
/// - `source_len`: Length of the compressed data.
/// - `long_lived_allocator`: The allocator to use for long-lived memory allocation.
/// - `short_lived_allocator`: The allocator to use for short-lived memory allocation.
///
/// # Returns
/// Number of bytes written to `destination`.
///
/// # Safety
///
/// It's safe as long as `dest` has sufficient length (max length: [`crate::util::prs_calculate_max_compressed_size`])
/// and the remaining parameters are valid.
pub unsafe fn prs_compress<L: Allocator + Copy, S: Allocator + Copy>(
    source: *const u8,
    mut dest: *mut u8,
    source_len: usize,
    long_lived_allocator: L,
    short_lived_allocator: S,
) -> usize {
    let orig_dest = dest as usize;

    // Write first control byte.
    let mut last_init_covered_all = false;
    let mut control_byte_ptr = reserve_control_byte(&mut dest);
    let mut control_bit_position = 0;
    let mut source_ofs = 0;
    let mut dict = CompDict::new_in(WINDOW_SIZE, long_lived_allocator, short_lived_allocator);

    // First byte is always a direct encode, so we can encode it before looping,
    // doing this here saves a branch in lz77_get_longest_match, improving perf.
    if source_len > 0 {
        append_control_bit(
            1,
            &mut dest,
            &mut control_bit_position,
            &mut control_byte_ptr,
        );
        append_byte(*source, &mut dest);
        source_ofs += 1;
    }

    // Loop through all the bytes, as long as there are less than COPY_MAX_LENGTH bytes left.
    // We eliminate a branch inside lz77_get_longest_match by doing this, saving a bit of perf.
    let fast_processing_end = source_len.saturating_sub(COPY_MAX_LENGTH as usize);
    while source_ofs < fast_processing_end {
        let window_start = source_ofs.saturating_sub(MAX_OFFSET);
        let window_end = window_start + WINDOW_SIZE;
        let window_end = if window_end >= source_len {
            last_init_covered_all = true;
            source_len
        } else {
            window_end
        };
        let window_slice =
            slice::from_raw_parts(source.add(window_start), window_end - window_start);
        dict.init(window_slice, window_start);

        // Process the current window.
        let fast_limit = window_end.min(fast_processing_end);
        while source_ofs < fast_limit {
            let mut result = lz77_get_longest_match_fast::<CompressParameters, L, S>(
                &mut dict, source, source_ofs,
            );

            // Lazy matching: inner loop to chain deferrals without recomputation
            while result.length >= 2 && source_ofs + 1 < fast_limit {
                let next_result = lz77_get_longest_match_fast::<CompressParameters, L, S>(
                    &mut dict,
                    source,
                    source_ofs + 1,
                );

                if next_result.length > result.length {
                    // Emit literal and advance, reusing next_result
                    append_control_bit(
                        1,
                        &mut dest,
                        &mut control_bit_position,
                        &mut control_byte_ptr,
                    );
                    append_byte(*source.add(source_ofs), &mut dest);
                    source_ofs += 1;
                    result = next_result;
                } else {
                    break;
                }
            }

            encode_lz77_match(
                result,
                &mut dest,
                &mut control_bit_position,
                &mut control_byte_ptr,
                &mut source_ofs,
                source,
            );
        }
    }

    // Handle the remaining bytes.
    // We sub 1 because `lz77_get_longest_match` reads the next 2 bytes.
    // If our file happens to be 1 byte from the end, we can't read 2 bytes.
    if !last_init_covered_all {
        // Only reinitialize the dictionary if we haven't already covered the entire file
        let window_start = source_ofs.saturating_sub(MAX_OFFSET);
        let window_slice =
            slice::from_raw_parts(source.add(window_start), source_len - window_start);
        dict.init(window_slice, window_start);
    }

    let slow_limit = source_len.saturating_sub(1);
    while source_ofs < slow_limit {
        let mut result = lz77_get_longest_match_slow::<CompressParameters, L, S>(
            &mut dict, source, source_len, source_ofs,
        );

        // Lazy matching for slow path
        while result.length >= 2 && source_ofs + 1 < slow_limit {
            let next_result = lz77_get_longest_match_slow::<CompressParameters, L, S>(
                &mut dict,
                source,
                source_len,
                source_ofs + 1,
            );

            if next_result.length > result.length {
                // Emit literal and advance, reusing next_result
                append_control_bit(
                    1,
                    &mut dest,
                    &mut control_bit_position,
                    &mut control_byte_ptr,
                );
                append_byte(*source.add(source_ofs), &mut dest);
                source_ofs += 1;
                result = next_result;
            } else {
                break;
            }
        }

        encode_lz77_match(
            result,
            &mut dest,
            &mut control_bit_position,
            &mut control_byte_ptr,
            &mut source_ofs,
            source,
        );
    }

    // There is potentially one last remaining byte.
    if source_ofs == source_len.wrapping_sub(1) {
        append_control_bit(
            1,
            &mut dest,
            &mut control_bit_position,
            &mut control_byte_ptr,
        );
        append_byte(*source.add(source_ofs), &mut dest);
    }

    // Finish the PRS file
    append_control_bit(
        0,
        &mut dest,
        &mut control_bit_position,
        &mut control_byte_ptr,
    );
    append_control_bit(
        1,
        &mut dest,
        &mut control_bit_position,
        &mut control_byte_ptr,
    );

    append_byte(0x00, &mut dest);
    append_byte(0x00, &mut dest);

    dest as usize - orig_dest
}

#[inline(always)]
unsafe fn encode_lz77_match(
    result: Lz77Match,
    dest: &mut *mut u8,
    control_bit_position: &mut usize,
    control_byte_ptr: &mut *mut u8,
    source_ofs: &mut usize,
    source: *const u8,
) {
    // Check for short copy.
    if result.offset >= -SHORT_COPY_MAX_OFFSET
        && result.length >= SHORT_COPY_MIN_LEN
        && result.length <= SHORT_COPY_MAX_LEN
    {
        write_short_copy(dest, &result, control_bit_position, control_byte_ptr);
        *source_ofs += result.length;
    } else if result.length <= 2 {
        // Otherwise write a direct byte if we can't compress.
        append_control_bit(1, dest, control_bit_position, control_byte_ptr);
        append_byte(*source.add(*source_ofs), dest);
        *source_ofs += 1;
    } else {
        // Otherwise encode a long copy
        if result.length <= 9 {
            write_long_copy_small(dest, &result, control_bit_position, control_byte_ptr);
            *source_ofs += result.length;
        } else {
            write_long_copy_large(dest, &result, control_bit_position, control_byte_ptr);
            *source_ofs += result.length;
        }
    }
}

/// Writes a short copy (00 opcode), size 2-5, offset 1-256
#[inline(always)]
unsafe fn write_short_copy(
    dest: &mut *mut u8,
    result: &Lz77Match,
    control_bit_position: &mut usize,
    control_byte_ptr: &mut *mut u8,
) {
    let encoded_len = result.length - 2;

    // Write 00 opcode.
    append_control_bit(0, dest, control_bit_position, control_byte_ptr);
    append_control_bit(0, dest, control_bit_position, control_byte_ptr);

    // Pack the size with the second byte first.
    append_control_bit(
        ((encoded_len >> 1) & 1) as u8,
        dest,
        control_bit_position,
        control_byte_ptr,
    );

    append_control_bit(
        (encoded_len & 1) as u8,
        dest,
        control_bit_position,
        control_byte_ptr,
    );

    // Write the offset into the destination. As negative int, truncated to byte
    append_byte((result.offset & 0xFF) as u8, dest);
}

/// Writes a long copy (01 opcode), with small length. size 3-9, offset 1-8191
#[inline(always)]
unsafe fn write_long_copy_small(
    dest: &mut *mut u8,
    result: &Lz77Match,
    control_bit_position: &mut usize,
    control_byte_ptr: &mut *mut u8,
) {
    // Write 01 opcode.
    append_control_bit(0, dest, control_bit_position, control_byte_ptr);
    append_control_bit(1, dest, control_bit_position, control_byte_ptr);

    // Pack the value.
    let len = (result.length - 2) as isize;
    let ofs = result.offset << 3 & 0xFFF8;
    let packed = (ofs | len) as u16;

    // Write the packed value.
    append_u16_le(packed, dest);
}

/// Writes a long copy (01 opcode), with large length. size 1-256, offset 1-8191
#[inline(always)]
unsafe fn write_long_copy_large(
    dest: &mut *mut u8,
    result: &Lz77Match,
    control_bit_position: &mut usize,
    control_byte_ptr: &mut *mut u8,
) {
    // Write 01 opcode.
    append_control_bit(0, dest, control_bit_position, control_byte_ptr);
    append_control_bit(1, dest, control_bit_position, control_byte_ptr);

    // Pack the value.
    let packed = (result.offset << 3 & 0xFFF8) as u16;

    // Write the packed value.
    append_u16_le(packed, dest);
    append_byte((result.length - 1) as u8, dest);
}

/// Appends a control 'bit' to the current control byte.
/// If the control byte is full, it's appended to the destination.
///
/// # Parameters
///
/// - `bit`: The either 0 or 1 bit to be appended onto the control byte.
/// - `dest`: The destination where the compressed data goes.
/// - `control_bit_position`: The current bit position in the control byte.
/// - `control_byte`: The current control byte.
#[inline]
unsafe fn append_control_bit(
    bit: u8,
    dest: &mut *mut u8,
    control_bit_position: &mut usize,
    control_byte_ptr: &mut *mut u8,
) {
    // Reserve next control byte if necessary.
    if *control_bit_position >= 8 {
        *control_byte_ptr = reserve_control_byte(dest);
        *control_bit_position = 0;
    }

    // Append the current bit position and go to next position.
    **control_byte_ptr |= bit << *control_bit_position;
    *control_bit_position += 1;
}

/// Advances by one, and returns address of previous byte.
#[inline]
fn reserve_control_byte(dest: &mut *mut u8) -> *mut u8 {
    unsafe {
        **dest = 0; // Make suer it's zeroed in case user passes non-zeroed buffer.
        let result = *dest;
        *dest = dest.add(1);
        result
    }
}

/// Appends single byte to destination.
#[inline]
unsafe fn append_byte(value: u8, dest: &mut *mut u8) {
    **dest = value;
    *dest = dest.add(1);
}

/// Appends two bytes to the destination, little endian
#[inline]
unsafe fn append_u16_le(value: u16, dest: &mut *mut u8) {
    write_unaligned((*dest) as *mut u16, value.to_le());
    *dest = dest.add(2);
}

struct CompressParameters;
impl Lz77Parameters for CompressParameters {
    const MAX_OFFSET: usize = MAX_OFFSET;
    const MAX_LENGTH: usize = COPY_MAX_LENGTH as usize;
}
