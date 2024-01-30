use core::{cmp::min, slice};

use crate::impls::comp::comp_dict::CompDict;

/// Parameters
///
/// - `source`: A pointer to the compressed data.
/// - `destination`: A pointer to the decompressed data.
/// - `source_len`: Length of the compressed data.
///
/// # Returns
/// Number of bytes written to `destination`.
pub(crate) unsafe fn prs_compress(
    mut source: *const u8,
    mut dest: *mut u8,
    source_len: usize,
) -> usize {
    let mut dict = CompDict::new(slice::from_raw_parts(source, source_len));

    // Write first control byte.
    let mut control_byte = 0;
    let mut control_byte_ptr = reserve_control_byte(&mut dest);
    let mut control_bit_position = 0;
    let mut decompressed_size = 0;

    let end = source.add(source_len);
    while source < end {
        let mut match_length = 0;
    }

    todo!()
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
    control_byte: &mut *mut u8,
) {
    // Reserve next control byte if necessary.
    if *control_bit_position >= 8 {
        *control_byte = reserve_control_byte(dest);
        *control_bit_position = 0;
    }

    // Append the current bit position and go to next position.
    **control_byte |= bit << *control_bit_position;
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

unsafe fn lz77_match(source: *const u8, dest: *mut u8, source_len: usize, max_length: usize) {
    const MAX_MATCH_LENGTH: usize = 256;
    const SEARCH_BUFFER_SIZE: usize = 0x1FFF;

    // Min profitable match length == 2 (based on PRS encoding scheme).

    // Pointer to start of the search buffer.
    let start_ptr = source.sub(min(SEARCH_BUFFER_SIZE, max_length));
}
