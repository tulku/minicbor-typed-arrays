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
#[cfg(feature = "half")]
pub use half;
#[cfg(feature = "alloc")]
pub use typed_array::OwnedTypedArray;
pub use typed_array::{InvalidLength, Iter, TypedArray, TypedArrayRef};

use crate::tag::element_type_from_tag;

impl<C, Ctx> minicbor::Encode<Ctx> for TypedArray<C>
where
    C: AsRef<[u8]>,
{
    fn encode<W: minicbor::encode::Write>(
        &self,
        e: &mut minicbor::Encoder<W>,
        _ctx: &mut Ctx,
    ) -> Result<(), minicbor::encode::Error<W::Error>> {
        let tag = self.element_type().tag(self.endianness());
        e.tag(tag)?.bytes(self.as_bytes())?;
        Ok(())
    }
}

impl<'b, Ctx> minicbor::Decode<'b, Ctx> for TypedArray<&'b [u8]> {
    fn decode(
        d: &mut minicbor::Decoder<'b>,
        _ctx: &mut Ctx,
    ) -> Result<Self, minicbor::decode::Error> {
        let tag = d.tag()?;
        let (element_type, endianness) = element_type_from_tag(tag)?;
        let bytes = d.bytes()?;
        TypedArray::new(element_type, endianness, bytes).map_err(|_| {
            minicbor::decode::Error::message(
                "typed array byte length is not a multiple of element width",
            )
        })
    }
}

#[cfg(feature = "alloc")]
impl<'b, Ctx> minicbor::Decode<'b, Ctx> for TypedArray<alloc::vec::Vec<u8>> {
    fn decode(
        d: &mut minicbor::Decoder<'b>,
        _ctx: &mut Ctx,
    ) -> Result<Self, minicbor::decode::Error> {
        let tag = d.tag()?;
        let (element_type, endianness) = element_type_from_tag(tag)?;
        let bytes = d.bytes()?.to_vec();
        TypedArray::new(element_type, endianness, bytes).map_err(|_| {
            minicbor::decode::Error::message(
                "typed array byte length is not a multiple of element width",
            )
        })
    }
}
