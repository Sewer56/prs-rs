use super::common::{read_byte, read_two_le, retrieve_control_bit};

pub(crate) unsafe fn prs_decompress(mut source: *const u8, mut dest: *mut u8) -> usize {
    let mut control_byte = read_byte(&mut source);
    let mut current_bit_position = 0;
    let mut file_size = 0;

    loop {
        // Test for Direct Byte (Opcode 1)
        if retrieve_control_bit(&mut control_byte, &mut current_bit_position, &mut source) == 1 {
            *dest = read_byte(&mut source) as u8;
            dest = dest.add(1);
            file_size += 1;
            continue;
        }

        // Opcode 1 failed, now testing for Opcode 0X
        if retrieve_control_bit(&mut control_byte, &mut current_bit_position, &mut source) == 1 {
            // Test for Opcode 01
            // Append size of long copy, break if it's end of file.
            if decode_long_copy(&mut source, &mut dest, &mut file_size) {
                break;
            }
        } else {
            // Do Opcode 00
            decode_short_copy(
                &mut control_byte,
                &mut current_bit_position,
                &mut source,
                &mut dest,
                &mut file_size,
            );
        }
    }

    file_size
}

#[inline]
unsafe fn decode_long_copy(
    source: &mut *const u8,
    dest: &mut *mut u8,
    file_size: &mut usize,
) -> bool {
    // Opcode 01, length 2 - 256
    let ofs_bytes = read_two_le(source) as isize;
    if ofs_bytes == 0 {
        return true;
    }

    // Obtain the offset. (negative i32, truncated to u16)
    // We lost our negative sign when we originally wrote the offset, doing -0x2000 will restore it.
    let offset = (ofs_bytes >> 3) | -0x2000;

    // Perf:
    // Calculate offset first, because length is more 'local', it's used by the
    // loop, while ofs is only used once.
    let length = ofs_bytes as usize & 0b111;
    let length = if length == 0 {
        read_byte(source) + 1 // length: 2 - 256
    } else {
        length + 2 // length: 2 - 9
    };

    let dest_local = *dest; // hoist the variable for perf
    let src_addr = dest_local.add(offset as usize);
    for i in 0..length {
        *dest_local.add(i) = *src_addr.add(i);
    }

    *dest = dest_local.add(length);
    *file_size += length;
    false
}

#[inline]
unsafe fn decode_short_copy(
    control_byte: &mut usize,
    current_bit_position: &mut usize,
    source: &mut *const u8,
    dest: &mut *mut u8,
    file_size: &mut usize,
) {
    // Opcode 00, length 2-5
    let mut length = retrieve_control_bit(control_byte, current_bit_position, source) << 1;
    length |= retrieve_control_bit(control_byte, current_bit_position, source);
    length += 2;

    // Obtain the offset. (negative i32, truncated to byte)
    // We lost our sign when we originally wrote the offset, doing -0x100 will restore it.
    let offset = read_byte(source) as isize | -0x100; // negative

    // Copy from source to dest
    // LLVM is magical, it just optimises this knowing max length is 5.
    // I have no idea how, given complexity of everything, but it does.
    // This ends up being very nice unrolled code copy.
    let dest_local = *dest; // hoist the variable for perf
    let src_addr = dest_local.add(offset as usize);
    for i in 0..length {
        *dest_local.add(i) = *src_addr.add(i);
    }

    *dest = dest_local.add(length);
    *file_size += length;
}
