#![no_std]
#![feature(allocator_api)]

extern crate alloc;
#[cfg(feature = "std")]
extern crate std;

pub mod comp;
pub mod decomp;
pub mod util;

#[cfg(test)]
pub mod test_prelude;

#[cfg(feature = "c-exports")]
pub mod exports;

pub mod impls {
    pub mod comp {
        pub mod comp_dict;
        pub mod compress;
        pub mod lz77_matcher;
    }

    pub mod decomp {
        pub(crate) mod common;
        pub mod decompress;
        pub mod estimate;
    }
}

pub trait ReadOnlyPointerSrc {
    fn as_ptr(&self) -> *const u8;
}

impl ReadOnlyPointerSrc for &[u8] {
    fn as_ptr(&self) -> *const u8 {
        (*self).as_ptr()
    }
}

impl ReadOnlyPointerSrc for *const u8 {
    fn as_ptr(&self) -> *const u8 {
        *self
    }
}

pub trait MutablePointerSrc {
    fn as_mut_ptr(&mut self) -> *mut u8;
}

impl MutablePointerSrc for &mut [u8] {
    fn as_mut_ptr(&mut self) -> *mut u8 {
        (*self).as_mut_ptr()
    }
}

impl MutablePointerSrc for *mut u8 {
    fn as_mut_ptr(&mut self) -> *mut u8 {
        *self
    }
}
