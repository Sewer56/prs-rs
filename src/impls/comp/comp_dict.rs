extern crate alloc;
use alloc::boxed::Box;
use alloc::vec;
use alloc::vec::Vec;
use core::ptr::write;
use core::{alloc::Layout, mem::size_of, ptr::read_unaligned};

// Note: Usage of this dict can be improved to reduce memory usage at cost of low runtime overhead, however
// given the nature of the PRS format being used on pre-2010 games, and usually files under 100MiB,
// I don't consider it worthwhile to increase complexity (and reduce perf). If there's a memory constrained
// scenario, I'll consider it however.

/// An alias for the max allowed offset in this dictionary.
/// We use u32 to be more cache friendly by default, although that limits us to 2GiB files.
/// Replace this with 'u64' if you need to compress files larger than 2GiB. You will however double memory usage.
pub(crate) type MaxOffset = u32;

const MAX_U16: usize = 65536;

/// Dictionary for PRS compression.
///
/// This dictionary stores the locations of every single possible place that a specified 2-byte sequence
/// can be found, with the 2 byte combination being the dictionary 'key'. The values (locations) are
/// stored inside a `Vec` in the [`CompDictEntry`] struct in ascending order.
///
/// When the compressor is looking for longest match at given address, it will read the 2 bytes at the
/// address and use that as key [`CompDict::get_item`]. Then the offsets inside the returned entry
/// will be used to greatly speed up search.
pub(crate) struct CompDict {
    dict: Box<[CompDictEntry; MAX_U16]>, // 2MiB on x64, else 1MiB
}

/// An individual entry in the [Compression Dictionary][`CompDict`].
///
/// This is an index of 'current item' and accompanying `Vec` containing all offsets which start
/// with the 2 byte sequence associated with this [`CompDictEntry`] inside the [`CompDict`].
///
/// In the index we track the 'last item' we used, such that when we advance this entry, we can
/// find the next offset that fits inside the LZ77 search window in effectively O(1) time.
///
/// # Optimization Note
///
/// This item is 4*usize in size, which is 32 bytes on 64-bit systems, and 16 bytes otherwise.
/// (Vec is 3*usize).
///
/// This is a good size, if the [`CompDict`] is allocated on a multiple of cache line size (64-bytes on x86),
/// no items will span cache line boundaries. Making things pretty cache efficient.
#[derive(Clone)]
pub(crate) struct CompDictEntry {
    items: Vec<MaxOffset>,
    current_item: usize,
}

impl CompDict {
    /// Create a new [`CompDict`] and initialize its entries.
    ///
    /// # Parameters
    /// - `freq_table` The frequency table for the data to be processed.
    ///                You can get this by calling [`CompDict::create_frequency_table`].
    pub(crate) fn new(freq_table: Box<[MaxOffset; MAX_U16]>) -> CompDict {
        unsafe {
            // Define the layout for our dictionary.
            // We align 64 to match the cache line size on x86.
            let layout =
                Layout::from_size_align_unchecked(size_of::<[CompDictEntry; MAX_U16]>(), 64);

            // Allocate the array on the heap
            let ptr = alloc::alloc::alloc(layout);

            // Initialize each item
            let dict_entry_ptr = ptr as *mut CompDictEntry;
            for i in 0..MAX_U16 {
                core::ptr::write(
                    // skip deallocating existing nonexisting items
                    dict_entry_ptr.add(i),
                    CompDictEntry {
                        items: Vec::with_capacity(freq_table[i] as usize),
                        current_item: 0,
                    },
                );
            }

            let dict = Box::<[CompDictEntry; MAX_U16]>::from_raw(ptr.cast());
            CompDict { dict }
        }
    }

    /// Creates a frequency table for the given data.
    ///
    /// # Parameters
    /// - `data`: The data to create the frequency table from.
    pub(crate) fn create_frequency_table(data: &[u8]) -> Box<[MaxOffset; MAX_U16]> {
        unsafe {
            // This actually has no overhead.
            let mut result: Box<[MaxOffset; MAX_U16]> =
                vec![0; MAX_U16].into_boxed_slice().try_into().unwrap();

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
    }

    /// Create a new [`CompDict`] from a given slice of bytes.
    ///
    /// # Parameters
    ///
    /// - `data`: The data to create the dictionary from.
    pub(crate) unsafe fn create(data: &[u8]) -> CompDict {
        let freq_table = Self::create_frequency_table(data);
        let mut dict = CompDict::new(freq_table);

        // Iterate over the data, and add each 2-byte sequence to the dictionary.
        let data_ptr = data.as_ptr();
        let data_ofs_max = data.len() - 1;
        let mut data_ofs = 0;
        while data_ofs < data_ofs_max {
            // LLVM successfully unrolls this
            dict.add_item(
                data_ofs as MaxOffset,
                read_unaligned(data_ptr.add(data_ofs) as *const u16),
                true,
            );
            data_ofs += 1;
        }

        dict
    }

    /// Adds an item to the Compression Dictionary [`CompDict`].
    pub(crate) fn add_item(&mut self, offset: MaxOffset, key: u16, is_unchecked: bool) {
        let entry = unsafe { &mut self.dict.get_unchecked_mut(key as usize) };
        // Constant folded by LLVM
        if is_unchecked {
            push_unchecked(&mut entry.items, offset);
        } else {
            entry.items.push(offset);
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
    /// # Remarks & Safety
    ///
    /// It is assumed that [`Self::get_item`] will always be called in a sequential manner.
    /// Calling it out of order will result in undefined behaviour and most likely out of bounds reads.
    pub(crate) unsafe fn get_item(
        &mut self,
        key: u16,
        min_ofs: usize,
        max_ofs: usize,
    ) -> &[MaxOffset] {
        let entry = &mut self.dict[key as usize];

        // Note that not checking `entry.items.len()` is technically unsafe, but assuming
        // this method is correctly used, it's ok.
        while *entry.items.get_unchecked(entry.current_item) < min_ofs as MaxOffset {
            entry.current_item += 1;
        }

        // Find the index of the first item that exceeds max_ofs
        let end_index = entry
            .items
            .iter()
            .position(|&offset| offset > max_ofs as MaxOffset)
            .unwrap_or(entry.items.len());

        entry.items.get_unchecked(entry.current_item..end_index)
    }
}

fn push_unchecked<T>(vec: &mut Vec<T>, value: T) {
    unsafe {
        let len = vec.len();
        write(vec.as_mut_ptr().add(len), value);
        vec.set_len(len + 1);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn can_add_item() {
        // Add an item with key 0x4141 and ensure it exists.
        let boxed_slice = vec![0; MAX_U16].into_boxed_slice().try_into().unwrap();
        let mut comp_dict = CompDict::new(boxed_slice);
        comp_dict.add_item(0, 0x4141, false);

        let entry = &comp_dict.dict[0x4141];
        assert_eq!(entry.items, vec![0]);
    }

    #[test]
    fn can_get_item() {
        unsafe {
            // Add multiple offsets at the same key, and ensure they are returned.
            let boxed_slice = vec![0; MAX_U16].into_boxed_slice().try_into().unwrap();
            let mut comp_dict = CompDict::new(boxed_slice);
            comp_dict.add_item(0, 0x4141, false);
            comp_dict.add_item(1, 0x4141, false);
            comp_dict.add_item(2, 0x4141, false);
            comp_dict.add_item(3, 0x4141, false);
            comp_dict.add_item(4, 0x4141, false);
            comp_dict.add_item(5, 0x4141, false);
            comp_dict.add_item(6, 0x4141, false);
            comp_dict.add_item(7, 0x4141, false);
            comp_dict.add_item(8, 0x4141, false);
            comp_dict.add_item(9, 0x4141, false);

            let result = comp_dict.get_item(0x4141, 1, 2);
            assert_eq!(&[1, 2], result);

            // Ensure pointer was advanced
            assert_eq!(comp_dict.dict[0x4141].current_item, 1);

            // Access the next in sequence, and ensure it was correctly advanced.
            let result = comp_dict.get_item(0x4141, 2, 3);
            assert_eq!(&[2, 3], result);
            assert_eq!(comp_dict.dict[0x4141].current_item, 2);

            // Change in max offset shouldn't change current item
            let result = comp_dict.get_item(0x4141, 2, 9);
            assert_eq!(&[2, 3, 4, 5, 6, 7, 8, 9], result);
            assert_eq!(comp_dict.dict[0x4141].current_item, 2);
        }
    }

    #[test]
    fn can_create_dict() {
        unsafe {
            let data = &[0x41, 0x42, 0x43];
            let comp_dict = CompDict::create(data);

            // Prevent dead code elimination.
            assert!(
                comp_dict.dict.len() > 0,
                "CompDict was not created correctly"
            );
            assert_eq!(comp_dict.dict[0x4241_u16.to_le() as usize].items, vec![0]);
            assert_eq!(comp_dict.dict[0x4342_u16.to_le() as usize].items, vec![1]);
        }
    }
}
