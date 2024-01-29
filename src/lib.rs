#![cfg_attr(not(test), no_std)]
#![feature(allocator_api)]

pub mod comp;
pub mod decomp;
pub mod exports;
pub mod util;

pub mod impls {
    pub mod comp {
        pub mod comp_dict;
        pub mod compress;
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
