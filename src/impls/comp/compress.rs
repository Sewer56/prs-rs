use super::lz77_matcher::Lz77Match;
use crate::impls::comp::{comp_dict::CompDict, lz77_matcher::lz77_get_longest_match};
use core::{ptr::write_unaligned, slice};

const SHORT_COPY_MAX_LENGTH: isize = 0x100;
const SHORT_COPY_MAX_OFFSET: usize = 5;
const SHORT_COPY_MIN_OFFSET: usize = 2;

/// Parameters
///
/// - `source`: A pointer to the compressed data.
/// - `destination`: A pointer to the decompressed data.
/// - `source_len`: Length of the compressed data.
///
/// # Returns
/// Number of bytes written to `destination`.
///
/// # Safety
///
/// It's safe as long as `dest` has sufficient length (max length: [`crate::util::prs_calculate_max_decompressed_size`])
/// and the remaining parameters are valid.
pub unsafe fn prs_compress(source: *const u8, mut dest: *mut u8, source_len: usize) -> usize {
    let orig_dest = dest as usize;
    let mut dict = CompDict::new(slice::from_raw_parts(source, source_len));

    // Write first control byte.
    let mut control_byte_ptr = reserve_control_byte(&mut dest);
    let mut control_bit_position = 0;
    let mut source_ofs = 0;

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

    // Loop through remaining bytes.
    while source_ofs < source_len {
        // Get longest match.
        let result =
            lz77_get_longest_match(&mut dict, source, source_len, source_ofs, 0x1FFF, 0x100);

        // Check for short copy.
        if result.offset >= -SHORT_COPY_MAX_LENGTH
            && result.length >= SHORT_COPY_MIN_OFFSET
            && result.length <= SHORT_COPY_MAX_OFFSET
        {
            write_short_copy(
                &mut dest,
                &result,
                &mut control_bit_position,
                &mut control_byte_ptr,
            );
            source_ofs += result.length;
        } else if result.length <= 2 {
            // Otherwise write a direct byte if we can't compress.
            append_control_bit(
                1,
                &mut dest,
                &mut control_bit_position,
                &mut control_byte_ptr,
            );
            append_byte(*source.add(source_ofs), &mut dest);
            source_ofs += 1;
        } else {
            // Otherwise encode a long copy
            if result.length <= 9 {
                write_long_copy_small(
                    &mut dest,
                    &result,
                    &mut control_bit_position,
                    &mut control_byte_ptr,
                );
                source_ofs += result.length;
            } else {
                write_long_copy_large(
                    &mut dest,
                    &result,
                    &mut control_bit_position,
                    &mut control_byte_ptr,
                );
                source_ofs += result.length;
            }
        }
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
