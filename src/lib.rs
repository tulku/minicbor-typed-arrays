#![cfg_attr(not(test), no_std)]

#[cfg(feature = "alloc")]
extern crate alloc;

mod element;
mod endianness;
mod tag;
mod typed_array;

pub use endianness::Endianness;
