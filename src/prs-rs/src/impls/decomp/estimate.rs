use super::common::{advance_byte, read_byte, read_two_le, retrieve_control_bit};

pub(crate) unsafe fn prs_calculate_decompressed_size_impl(mut source: *const u8) -> usize {
    let mut control_byte = read_byte(&mut source);
    let mut current_bit_position = 0;
    let mut file_size = 0;

    loop {
        // Test for Direct Byte (Opcode 1)
        if retrieve_control_bit(&mut control_byte, &mut current_bit_position, &mut source) == 1 {
            source = source.add(1);
            file_size += 1;
            continue;
        }

        // Opcode 1 failed, now testing for Opcode 0X
        if retrieve_control_bit(&mut control_byte, &mut current_bit_position, &mut source) == 1 {
            // Test for Opcode 01
            // Append size of long copy, break if it's end of file.
            if decode_long_copy(&mut source, &mut file_size) {
                break;
            }
        } else {
            // Do Opcode 00
            decode_short_copy(
                &mut control_byte,
                &mut current_bit_position,
                &mut source,
                &mut file_size,
            );
        }
    }

    file_size
}

#[inline]
unsafe fn decode_long_copy(source: &mut *const u8, file_size: &mut usize) -> bool {
    let offset = read_two_le(source);
    if offset == 0 {
        return true;
    }

    let length = offset & 0b111;
    let length = if length == 0 {
        read_byte(source) + 1
    } else {
        length + 2
    };

    *file_size += length;
    false
}

#[inline]
unsafe fn decode_short_copy(
    control_byte: &mut usize,
    current_bit_position: &mut usize,
    source: &mut *const u8,
    file_size: &mut usize,
) {
    let mut length = retrieve_control_bit(control_byte, current_bit_position, source) << 1;
    length |= retrieve_control_bit(control_byte, current_bit_position, source);
    length += 2;

    // Simulate reading the offset
    advance_byte(source);

    *file_size += length;
}
