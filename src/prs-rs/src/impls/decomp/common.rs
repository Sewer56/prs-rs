use core::ptr::read_unaligned;

#[inline]
pub(crate) unsafe fn read_byte(source: &mut *const u8) -> usize {
    let byte = *source;
    *source = source.add(1);
    *byte as usize
}

#[inline]
pub(crate) unsafe fn read_two_le(source: &mut *const u8) -> usize {
    let bytes = read_unaligned(*source as *const u16);
    *source = source.add(2);
    u16::from_le(bytes) as usize
}

#[inline]
pub(crate) unsafe fn advance_byte(source: &mut *const u8) {
    *source = source.add(1);
}

#[inline]
pub(crate) unsafe fn retrieve_control_bit(
    control_byte: &mut usize,
    current_bit_position: &mut usize,
    source: &mut *const u8,
) -> usize {
    // cold path
    // unfortunately likely/unlikely does not affect codegen much here.
    if *current_bit_position >= 8 {
        *control_byte = read_byte(source);
        *current_bit_position = 0;
    }

    let return_value = *control_byte & 0x01;
    *control_byte >>= 1;
    *current_bit_position += 1;

    return_value
}
