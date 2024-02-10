extern crate alloc;
use alloc::alloc::{alloc, dealloc};
use alloc::boxed::Box;
use alloc::vec;
use core::ptr::{read, write, NonNull};
use core::slice;
use core::{alloc::Layout, mem::size_of, ptr::read_unaligned};

// Note: Usage of this dict can be improved to reduce memory usage at cost of low runtime overhead, however
// given the nature of the PRS format being used on pre-2010 games, and usually files under 100MiB,
// I don't consider it worthwhile to increase complexity (and reduce perf). If there's a memory constrained
// scenario, I'll consider it however.

// In any case, this dictionary, when applied over a whole file, causes the compression operation to take
// 4xFileSize amount of RAM.

type MaxOffset = u32;
const MAX_U16: usize = 65536;
const ALLOC_ALIGNMENT: usize = 64; // x86 cache line

// Round up to next multiple of ALLOC_ALIGNMENT
const DICTIONARY_PADDING: usize =
    (ALLOC_ALIGNMENT - (size_of::<[CompDictEntry; MAX_U16]>() % ALLOC_ALIGNMENT)) % ALLOC_ALIGNMENT;

/// Dictionary for PRS compression.
///
/// This dictionary stores the locations of every single possible place that a specified 2-byte sequence
/// can be found, with the 2 byte combination being the dictionary 'key'. The values (locations) are
/// stored inside a shared buffer, where [`CompDictEntry`] dictates the file offsets of the locations
/// which start with this 2 byte combination. The items are stored in ascending order.
///
/// When the compressor is looking for longest match at given address, it will read the 2 bytes at the
/// address and use that as key [`CompDict::get_item`]. Then the offsets inside the returned entry
/// will be used to greatly speed up search.
pub struct CompDict {
    /// Our memory allocation is here.
    /// Layout:
    /// - [CompDictEntry; MAX_U16] (dict), constant size
    /// - [MaxOffset; file_num_bytes] (offsets), variable size. This buffer stores offsets of all items of 2 byte combinations.
    buf: NonNull<MaxOffset>,
    alloc_length: usize, // length of data that 'dict' and 'offsets' were made with
}

impl Drop for CompDict {
    fn drop(&mut self) {
        unsafe {
            // dealloc buffer and box
            let layout = Layout::from_size_align_unchecked(
                self.alloc_length,
                ALLOC_ALIGNMENT,
            );
            dealloc(self.buf.as_ptr() as *mut u8, layout);
        }
    }
}

/// An entry in [Compression Dictionary][`CompDict`].
///
/// This has pointer to current 'last min offset' [`CompDictEntry::last_read_item`] in [`CompDict::offsets`] allocation,
/// and pointer to last offset for the current 2 byte key.
///
/// Last min offset [`CompDictEntry::last_read_item`] is advanced as items are sequentially read,
/// i.e. when [`CompDict::get_item`] is called. This offset corresponds to the first item which had
/// offset greater than `min_offset` parameter of last [`CompDict::get_item`] call.
///
/// When compressing, this means we can find next matching offset in LZ77 search window
/// in (effectively) O(1) time.
#[derive(Clone)]
pub struct CompDictEntry {
    /// Address of the last minimum offset from previous call to [`CompDict::get_item`].
    last_read_item: *mut MaxOffset,
    /// Address of the last maximum offset from previous call to [`CompDict::get_item`].
    last_read_item_max: *mut MaxOffset,
    /// Item after last item within the [`CompDict::offsets`] allocation belonging to this entry.
    last_item: *mut MaxOffset,
}

impl CompDict {
    /// Create a new [`CompDict`] from a given slice of bytes.
    ///
    /// # Parameters
    ///
    /// - `data`: The data to create the dictionary from.
    pub(crate) unsafe fn new(data: &[u8]) -> CompDict {
        let freq_table = Self::create_frequency_table(data);

        // Preallocate the buffer Dict Entries and Offsets
        let entry_section_len = size_of::<[CompDictEntry; MAX_U16]>(); // constant
        let offset_section_len = size_of::<MaxOffset>() * data.len();
        let alloc_size = entry_section_len + DICTIONARY_PADDING + offset_section_len;

        let layout = Layout::from_size_align_unchecked(alloc_size, ALLOC_ALIGNMENT);
        let buf = alloc(layout);

        let dict_entry_ptr = buf as *mut CompDictEntry;
        let max_ofs_ptr = buf.add(entry_section_len + DICTIONARY_PADDING) as *mut MaxOffset;

        // We will use this later to populate the dictionary.
        // This stores the location we start inserting offsets for each 2 byte sequence.
        let mut dict_insert_entry_ptrs = Box::<[*mut MaxOffset; MAX_U16]>::new_uninit();

        // Initialize all CompDictEntries
        let mut cur_ofs_addr = max_ofs_ptr;
        let mut cur_dict_entry = dict_entry_ptr;
        let mut cur_freq_tbl_entry = freq_table.as_ptr();
        let mut cur_ofs_insert_ptr = dict_insert_entry_ptrs.as_mut_ptr() as *mut *mut MaxOffset;
        let max_dict_entry = cur_dict_entry.add(MAX_U16);

        while cur_dict_entry < max_dict_entry {
            let num_items = *cur_freq_tbl_entry;
            *cur_ofs_insert_ptr = cur_ofs_addr;

            write(
                cur_dict_entry,
                CompDictEntry {
                    last_read_item: cur_ofs_addr,
                    last_read_item_max: cur_ofs_addr,
                    last_item: cur_ofs_addr.add(num_items as usize),
                },
            );

            cur_ofs_addr = cur_ofs_addr.add(num_items as usize);
            cur_freq_tbl_entry = cur_freq_tbl_entry.add(1);
            cur_dict_entry = cur_dict_entry.add(1);
            cur_ofs_insert_ptr = cur_ofs_insert_ptr.add(1);
        }

        let mut dict_insert_entry_ptrs = dict_insert_entry_ptrs.assume_init();

        // Iterate over the data, and add each 2-byte sequence to the dictionary.
        #[cfg(not(target_pointer_width = "64"))]
        {
            let data_ptr_start = data.as_ptr();
            let mut data_ptr = data.as_ptr();
            let data_ptr_max = data.as_ptr().add(data.len() - 1);
            debug_assert!(data.len() as MaxOffset <= MaxOffset::MAX);

            while data_ptr < data_ptr_max {
                let key = read_unaligned(data_ptr as *const u16);
                let insert_entry_ptr = dict_insert_entry_ptrs.as_mut_ptr().add(key as usize);

                // Insert the offset into the dictionary
                **insert_entry_ptr = data_ptr.sub(data_ptr_start as usize) as MaxOffset; // set offset
                *insert_entry_ptr = (*insert_entry_ptr).add(1); // advance to next entry

                data_ptr = data_ptr.add(1);
            }
        }

        #[cfg(target_pointer_width = "64")]
        {
            let mut data_ofs = 0;
            let data_len = data.len();

            while data_ofs < data_len.saturating_sub(16) {
                // Doing a lot of the `data.as_ptr().add()` is ugly, but it makes LLVM do a better job.
                let chunk = read(data.as_ptr().add(data_ofs) as *const u64);

                // Process every 16-bit sequence starting at each byte within the 64-bit chunk
                for shift in 0..7 {
                    // Successfully unrolled by LLVM
                    let key = ((chunk >> (shift * 8)) & 0xFFFF) as u16;
                    let insert_entry_ptr = dict_insert_entry_ptrs.as_mut_ptr().add(key as usize);

                    **insert_entry_ptr = (data.as_ptr().add(data_ofs + shift) as usize
                        - data.as_ptr() as usize)
                        as MaxOffset;
                    *insert_entry_ptr = (*insert_entry_ptr).add(1);
                }

                // Handle the 16-bit number that spans the boundary between this chunk and the next
                // Note: LLVM puts next_chunk in register and reuses it for next loop iteration (under x64), nothing special to do here.
                let next_chunk = read(data.as_ptr().add(data_ofs + 8) as *const u64);
                let next_chunk_byte = (next_chunk & 0xFF) << 8;
                let key = ((chunk >> 56) | next_chunk_byte) as u16;
                let insert_entry_ptr = dict_insert_entry_ptrs.as_mut_ptr().add(key as usize);

                **insert_entry_ptr = (data.as_ptr().add(data_ofs + 7) as usize
                    - data.as_ptr() as usize) as MaxOffset;
                *insert_entry_ptr = (*insert_entry_ptr).add(1);

                data_ofs += 8;
            }

            // Process any remaining bytes in the data.
            while data_ofs < data_len.saturating_sub(1) {
                let key = read_unaligned(data.as_ptr().add(data_ofs) as *const u16);
                let insert_entry_ptr = dict_insert_entry_ptrs.as_mut_ptr().add(key as usize);

                **insert_entry_ptr =
                    (data.as_ptr().add(data_ofs) as usize - data.as_ptr() as usize) as MaxOffset;
                *insert_entry_ptr = (*insert_entry_ptr).add(1);
                data_ofs += 1;
            }
        }

        CompDict {
            buf: NonNull::new_unchecked(buf as *mut MaxOffset),
            alloc_length: alloc_size,
        }
    }

    /// Creates a frequency table for the given data.
    ///
    /// # Parameters
    /// - `data`: The data to create the frequency table from.
    pub(crate) unsafe fn create_frequency_table(data: &[u8]) -> Box<[MaxOffset; MAX_U16]> {
        // This actually has no overhead.
        let mut result: Box<[MaxOffset; MAX_U16]> =
            vec![0; MAX_U16].into_boxed_slice().try_into().unwrap();

        #[cfg(not(target_pointer_width = "64"))]
        {
            // Iterate over the data, and add each 2-byte sequence to the dictionary.
            let data_ptr = data.as_ptr();
            let data_ofs_max = data.len() - 1;
            let mut data_ofs = 0;
            while data_ofs < data_ofs_max {
                // LLVM successfully unrolls this
                let index = read_unaligned(data_ptr.add(data_ofs) as *const u16);
                result[index as usize] += 1;
                data_ofs += 1;
            }

            result
        }

        #[cfg(target_pointer_width = "64")]
        {
            let data_len = data.len();
            let mut data_ofs = 0;

            while data_ofs < data_len.saturating_sub(16) {
                let chunk = read_unaligned(data.as_ptr().add(data_ofs) as *const u64);

                // Process every 16-bit sequence starting at each byte within the 64-bit chunk
                for shift in 0..7 {
                    let index = ((chunk >> (shift * 8)) & 0xFFFF) as u16;
                    result[index as usize] += 1;
                }

                // Handle the 16-bit number that spans the boundary between this chunk and the next
                // Note: LLVM puts next_chunk in register and reuses it for next loop iteration (under x64), nothing special to do here.
                let next_chunk = read_unaligned(data.as_ptr().add(data_ofs + 8) as *const u64);
                let key = ((chunk >> 56) | ((next_chunk & 0xFF) << 8)) as u16;
                result[key as usize] += 1;

                data_ofs += 8;
            }

            // Process any remaining bytes in the data.
            while data_ofs < data_len.saturating_sub(1) {
                let index = read_unaligned(data.as_ptr().add(data_ofs) as *const u16);
                result[index as usize] += 1;
                data_ofs += 1;
            }

            result
        }
    }

    /// Retrieves the dictionary entries section of this [`CompDict`].
    pub fn get_dict_mut(&mut self) -> &mut [CompDictEntry; MAX_U16] {
        unsafe {
            let first_item = self.buf.as_ptr() as *mut CompDictEntry;
            &mut *(first_item as *mut [CompDictEntry; MAX_U16])
        }
    }

    /// Returns a slice of offsets for the given key which are greater than or equal to `min_ofs`
    /// and less than or equal to `max_ofs`.
    ///
    /// # Parameters
    ///
    /// - `key`: The key to search for.
    /// - `min_ofs`: The minimum offset returned in the slice.
    /// - `max_ofs`: The maximum offset returned in the slice.
    ///
    /// # Safety
    ///
    /// This function is unsafe as it operates on raw pointers.
    pub(crate) unsafe fn get_item(
        &mut self,
        key: u16,
        min_ofs: usize,
        max_ofs: usize,
    ) -> &[MaxOffset] {
        // Ensure that the key is within the bounds of the dictionary.
        debug_assert!(key as usize <= MAX_U16, "Key is out of range!");

        let entry = &mut self.get_dict_mut()[key as usize];
        let mut cur_last_read_item = entry.last_read_item;

        // Advance the 'last_read_item' pointer to the first offset greater than or equal to min_ofs
        while cur_last_read_item < entry.last_item && *cur_last_read_item < min_ofs as MaxOffset {
            cur_last_read_item = cur_last_read_item.add(1);
        }
        entry.last_read_item = cur_last_read_item;

        // Find the end of the range - the first offset greater than max_ofs
        // TODO: Try last read max item.
        let mut end = entry.last_read_item_max;
        while end < entry.last_item && *end <= max_ofs as MaxOffset {
            end = end.add(1);
        }
        entry.last_read_item_max = end;

        // Create a slice from the updated range
        slice::from_raw_parts(
            cur_last_read_item,
            end.offset_from(cur_last_read_item) as usize,
        )
    }
}

impl CompDictEntry {
    /// Returns a slice of offsets between `last_read_item` and `last_item`.
    ///
    /// # Safety
    ///
    /// This function is unsafe as it operates on raw pointers.
    /// The caller must ensure that `last_read_item` and `last_item` are valid.
    #[cfg(test)]
    pub unsafe fn get_items(&mut self) -> &[MaxOffset] {
        // Calculate the length of the slice by finding the distance between the pointers.
        let length = self.last_item.offset_from(self.last_read_item) as usize;

        // Create and return a slice from the raw pointers.
        slice::from_raw_parts(self.last_read_item, length)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn can_create_dict() {
        unsafe {
            let data = &[0x41, 0x42, 0x43, 0x41, 0x41, 0x41, 0x41, 0x41, 0x41];
            let mut comp_dict = CompDict::new(data);

            // Assert that the items were correctly inserted.
            assert_eq!(
                comp_dict.get_dict_mut()[0x4241_u16.to_le() as usize].get_items(),
                &[0]
            );
            assert_eq!(
                comp_dict.get_dict_mut()[0x4342_u16.to_le() as usize].get_items(),
                &[1]
            );

            // Ensure we can get a slice.
            let result = comp_dict.get_item(0x4141, 3, 4);
            assert_eq!(&[3, 4], result);

            // Access the next in sequence, and ensure it was correctly advanced.
            let result = comp_dict.get_item(0x4141, 4, 5);
            assert_eq!(&[4, 5], result);
            assert_eq!(*comp_dict.get_dict_mut()[0x4141].last_read_item, 4);

            // Access beyond end of sequence
            let result = comp_dict.get_item(0x4141, 5, 99);
            assert_eq!(&[5, 6, 7], result);
        }
    }
}
