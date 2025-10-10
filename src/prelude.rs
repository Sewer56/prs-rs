//! Allocator API prelude with conditional compilation support
//! 
//! This module provides allocator-related types and traits that work on both
//! stable and nightly Rust channels. When the "nightly" feature is enabled,
//! it uses the native allocator API from std/core. Otherwise, it uses the
//! allocator-api2 crate.

#[cfg(not(feature = "nightly"))]
pub use allocator_api2::alloc::Allocator;

#[cfg(feature = "nightly")]
pub use std::alloc::Allocator;

#[cfg(not(feature = "nightly"))]
pub use allocator_api2::alloc::Global;

#[cfg(feature = "nightly")]
pub use std::alloc::Global;

#[cfg(not(feature = "nightly"))]
pub use allocator_api2::boxed::Box;

#[cfg(feature = "nightly")]
pub use std::boxed::Box;

#[cfg(not(feature = "nightly"))]
pub use allocator_api2::alloc::Layout;

#[cfg(feature = "nightly")]
pub use std::alloc::Layout;