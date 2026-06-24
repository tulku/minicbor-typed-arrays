#![cfg_attr(not(test), no_std)]

#[cfg(feature = "alloc")]
extern crate alloc;
#[cfg(feature = "std")]
extern crate std;

mod element;
mod endianness;
mod tag;
mod typed_array;

pub use element::{Element, ElementType, Scalar};
pub use endianness::Endianness;
pub use typed_array::{InvalidLength, Iter, TypedArray, TypedArrayRef};
#[cfg(feature = "alloc")]
pub use typed_array::OwnedTypedArray;
#[cfg(feature = "half")]
pub use half;
