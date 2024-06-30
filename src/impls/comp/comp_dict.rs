extern crate alloc;
use alloc::boxed::Box;
use core::alloc::Allocator;
use core::ptr::{write, NonNull};
use core::slice;
use core::{alloc::Layout, mem::size_of, ptr::read_unaligned};
use std::alloc::Global;

pub(crate) type MaxOffset = u32;
type FreqCountType = u32;
const MAX_U16: usize = 65536;
const ALLOC_ALIGNMENT: usize = 64; // x86 cache line

// Round up to next multiple of ALLOC_ALIGNMENT
const DICTIONARY_PADDING: usize =
    (ALLOC_ALIGNMENT - (size_of::<[CompDictEntry; MAX_U16]>() % ALLOC_ALIGNMENT)) % ALLOC_ALIGNMENT;

const ENTRY_SECTION_LEN: usize = size_of::<[CompDictEntry; MAX_U16]>();

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
pub struct CompDict<L: Allocator + Copy = Global, S: Allocator + Copy = Global> {
    /// Our memory allocation is here.
    /// Layout:
    /// - [CompDictEntry; MAX_U16] (dict), constant size
    /// - [MaxOffset; data_len_num_bytes] (offsets), variable size. This buffer stores offsets of all items of 2 byte combinations.
    buf: NonNull<u8>,
    alloc_length: usize, // length of data that 'dict' and 'offsets' were made with
    long_lived_allocator: L,
    short_lived_allocator: S,
}

impl<L: Allocator + Copy, S: Allocator + Copy> Drop for CompDict<L, S> {
    fn drop(&mut self) {
        unsafe {
            // dealloc buffer and box
            let layout = Layout::from_size_align_unchecked(self.alloc_length, ALLOC_ALIGNMENT);
            self.long_lived_allocator.deallocate(self.buf, layout);
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

impl<L: Allocator + Copy, S: Allocator + Copy> CompDict<L, S> {
    /// Create a new [`CompDict`] without initializing it.
    ///
    /// # Parameters
    ///
    /// - `data_len`: The length of the data that will be used to initialize the dictionary.
    /// - `long_lived_allocator`: The allocator to use for long-lived memory allocation.
    /// - `short_lived_allocator`: The allocator to use for short-lived memory allocation.
    #[inline(always)]
    pub fn new_in(data_len: usize, long_lived_allocator: L, short_lived_allocator: S) -> Self {
        unsafe {
            // constant
            let offset_section_len = size_of::<MaxOffset>() * data_len;
            let alloc_size = ENTRY_SECTION_LEN + DICTIONARY_PADDING + offset_section_len;

            let layout = Layout::from_size_align_unchecked(alloc_size, ALLOC_ALIGNMENT);
            let buf = long_lived_allocator.allocate(layout).unwrap();

            CompDict {
                buf: NonNull::new_unchecked(buf.as_ptr() as *mut u8),
                alloc_length: alloc_size,
                long_lived_allocator,
                short_lived_allocator,
            }
        }
    }

    /// Initialize the [`CompDict`] with the given data and offset.
    ///
    /// # Parameters
    ///
    /// - `data`: The data to create the dictionary from.
    /// - `offset`: The offset to add to the offsets in the dictionary.
    ///
    /// # Safety
    ///
    /// This function is unsafe as it operates on raw pointers and assumes that
    /// the `CompDict` has been properly allocated with enough space for `data`.
    #[inline(always)]
    pub unsafe fn init(&mut self, data: &[u8], offset: usize) {
        let dict_entry_ptr = self.buf.as_ptr() as *mut CompDictEntry;
        let max_ofs_ptr =
            self.buf
                .as_ptr()
                .add(ENTRY_SECTION_LEN + DICTIONARY_PADDING) as *mut MaxOffset;

        // We will use this later to populate the dictionary.
        // The `dict_insert_entry_ptrs` is a buffer which stores the pointer to the current location
        // where we need to insert the offset for a given 2 byte sequence (hence length MAX_U16).
        let alloc = self
            .short_lived_allocator
            .allocate(Layout::new::<[*mut MaxOffset; MAX_U16]>())
            .unwrap()
            .as_ptr() as *mut [*mut MaxOffset; MAX_U16];

        let mut dict_insert_entry_ptrs =
            Box::<[*mut MaxOffset; MAX_U16], S>::from_raw_in(alloc, self.short_lived_allocator);

        // dict_insert_entry_ptrs is now a Box, so it will be deallocated when it goes out of scope.

        // Initialize all CompDictEntries
        let freq_table = self.create_frequency_table(data);
        let mut cur_ofs_addr = max_ofs_ptr;
        let mut cur_dict_entry = dict_entry_ptr;
        let mut cur_freq_tbl_entry = freq_table.as_ptr();
        let mut cur_ofs_insert_ptr = dict_insert_entry_ptrs.as_mut_ptr();
        let max_dict_entry = cur_dict_entry.add(MAX_U16);

        // This loop initializes each CompDictEntry (ies) based on the frequency table.
        // It sets up the pointers for where the offsets for each 2-byte sequence will be stored.
        // This also populates `dict_insert_entry_ptrs` (via `cur_ofs_insert_ptr`) setting each
        // entry to the value of `cur_ofs_addr` (the current offset address).
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

        // The rest of the function is dedicated to actually populating the dictionary with offsets.
        // Here we do the following:
        // - Read Each 2 Byte Sequence
        // - Use 2 Byte Sequence as Key
        // - Gets insert location via `dict_insert_entry_ptrs` (**insert_entry_ptr)
        // - Advance insert location for given key (*insert_entry_ptr)

        // Iterate over the data, and add each 2-byte sequence to the dictionary.
        #[cfg(not(target_pointer_width = "64"))]
        {
            let data_ptr_start = data.as_ptr();
            let mut data_ptr = data.as_ptr();
            let data_ptr_max = data.as_ptr().add(data.len().saturating_sub(1));
            debug_assert!(data.len() as MaxOffset <= MaxOffset::MAX);

            while data_ptr < data_ptr_max {
                let key = read_unaligned(data_ptr as *const u16);
                let insert_entry_ptr = dict_insert_entry_ptrs.as_mut_ptr().add(key as usize);

                // Insert the offset into the dictionary
                **insert_entry_ptr = (data_ptr.sub(data_ptr_start as usize) as MaxOffset)
                    .wrapping_add(offset as MaxOffset);

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
                let chunk = read_unaligned(data.as_ptr().add(data_ofs) as *const u64);

                // Process every 16-bit sequence starting at each byte within the 64-bit chunk
                for shift in 0..7 {
                    // Successfully unrolled by LLVM
                    let key = ((chunk >> (shift * 8)) & 0xFFFF) as u16;
                    let insert_entry_ptr = dict_insert_entry_ptrs.as_mut_ptr().add(key as usize);

                    **insert_entry_ptr = ((data.as_ptr().add(data_ofs + shift) as usize
                        - data.as_ptr() as usize)
                        as MaxOffset)
                        .wrapping_add(offset as MaxOffset);

                    *insert_entry_ptr = (*insert_entry_ptr).add(1);
                }

                // Handle the 16-bit number that spans the boundary between this chunk and the next
                // Note: LLVM puts next_chunk in register and reuses it for next loop iteration (under x64), nothing special to do here.
                let next_chunk = read_unaligned(data.as_ptr().add(data_ofs + 8) as *const u64);
                let next_chunk_byte = (next_chunk & 0xFF) << 8;
                let key = ((chunk >> 56) | next_chunk_byte) as u16;
                let insert_entry_ptr = dict_insert_entry_ptrs.as_mut_ptr().add(key as usize);

                **insert_entry_ptr = ((data.as_ptr().add(data_ofs + 7) as usize
                    - data.as_ptr() as usize) as MaxOffset)
                    .wrapping_add(offset as MaxOffset);
                *insert_entry_ptr = (*insert_entry_ptr).add(1);

                data_ofs += 8;
            }

            // Process any remaining bytes in the data.
            while data_ofs < data_len.saturating_sub(1) {
                let key = read_unaligned(data.as_ptr().add(data_ofs) as *const u16);
                let insert_entry_ptr = dict_insert_entry_ptrs.as_mut_ptr().add(key as usize);

                **insert_entry_ptr = ((data.as_ptr().add(data_ofs) as usize
                    - data.as_ptr() as usize) as MaxOffset)
                    .wrapping_add(offset as MaxOffset);
                *insert_entry_ptr = (*insert_entry_ptr).add(1);
                data_ofs += 1;
            }
        }
    }

    /// Creates a frequency table for the given data.
    ///
    /// # Parameters
    /// - `data`: The data to create the frequency table from.
    pub(crate) unsafe fn create_frequency_table(&self, data: &[u8]) -> Box<[FreqCountType], S> {
        // This actually has no overhead.

        let result =
            Box::<[FreqCountType], S>::new_zeroed_slice_in(MAX_U16, self.short_lived_allocator);
        let mut result = result.assume_init();

        #[cfg(not(target_pointer_width = "64"))]
        {
            // Iterate over the data, and add each 2-byte sequence to the dictionary.
            let data_ptr = data.as_ptr();
            let data_ofs_max = data.len().saturating_sub(1);
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
    #[inline(always)]
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

    /// Retrieves the dictionary entries section of this [`CompDict`].
    pub fn get_dict_mut(&mut self) -> &mut [CompDictEntry; MAX_U16] {
        unsafe {
            let first_item = self.buf.as_ptr() as *mut CompDictEntry;
            &mut *(first_item as *mut [CompDictEntry; MAX_U16])
        }
    }
}

impl CompDict {
    /// Create a new [`CompDict`] without initializing it.
    ///
    /// # Parameters
    ///
    /// - `data_len`: The length of the data that will be used to initialize the dictionary.
    pub fn new(data_len: usize) -> Self {
        Self::new_in(data_len, Global, Global)
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
            let mut comp_dict = CompDict::new(data.len());
            comp_dict.init(data, 0);

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

    #[test]
    fn can_create_dict_with_offset() {
        unsafe {
            let data = &[0x41, 0x42, 0x43, 0x41, 0x41, 0x41, 0x41, 0x41, 0x41];
            let offset = 1000;
            let mut comp_dict = CompDict::new(data.len());
            comp_dict.init(data, offset);

            // Assert that the items were correctly inserted with the offset.
            assert_eq!(
                comp_dict.get_dict_mut()[0x4241_u16.to_le() as usize].get_items(),
                &[1000]
            );
            assert_eq!(
                comp_dict.get_dict_mut()[0x4342_u16.to_le() as usize].get_items(),
                &[1001]
            );

            // Ensure we can get a slice with offsets.
            let result = comp_dict.get_item(0x4141, 1003, 1004);
            assert_eq!(&[1003, 1004], result);

            // Access the next in sequence, and ensure it was correctly advanced.
            let result = comp_dict.get_item(0x4141, 1004, 1005);
            assert_eq!(&[1004, 1005], result);
            assert_eq!(*comp_dict.get_dict_mut()[0x4141].last_read_item, 1004);

            // Access beyond end of sequence
            let result = comp_dict.get_item(0x4141, 1005, 1099);
            assert_eq!(&[1005, 1006, 1007], result);
        }
    }
}
