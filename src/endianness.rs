use crate::{typed_array::TypedArray, typed_array_tag::Endianness};
#[cfg(feature = "half")]
use half::f16;
use std::mem::size_of;

pub trait EndiannessAware {
    fn desired_endianness(&self) -> Endianness;
    fn to_vec8<const C: usize, T: ToFromBytes<C>>(&self, array: &[T]) -> Vec<u8>;
}

pub trait ToFromBytes<const C: usize> {
    fn from_le(array: &[u8; C]) -> Self;
    fn from_be(array: &[u8; C]) -> Self;
    fn to_le(&self) -> [u8; C];
    fn to_be(&self) -> [u8; C];
}

macro_rules! impl_to_bytes {
    ($($t:ty),*) => {
        $(
            impl ToFromBytes<{size_of::<$t>()}> for $t {
                fn to_le(&self) -> [u8; size_of::<$t>()] {
                    self.to_le_bytes()
                }
                fn to_be(&self) -> [u8; size_of::<$t>()] {
                    self.to_be_bytes()
                }
                fn from_le(array: &[u8; size_of::<$t>()]) -> Self {
                    Self::from_le_bytes(*array)
                }
                fn from_be(array: &[u8; size_of::<$t>()]) -> Self {
                    Self::from_be_bytes(*array)
                }
            }
        )*
    };
}
impl_to_bytes!(u8, u16, u32, u64, i8, i16, i32, i64, f32, f64);
#[cfg(feature = "half")]
impl_to_bytes!(f16);

impl TypedArray {
    pub fn from_u8s<const C: usize, T: ToFromBytes<C>>(
        array: &[u8],
        endianness: &Endianness,
    ) -> Self
    where
        Self: std::iter::FromIterator<T>,
    {
        match endianness {
            Endianness::BigEndian => array
                .chunks_exact(size_of::<T>())
                .map(|data| T::from_be(data.try_into().unwrap()))
                .collect(),
            Endianness::LittleEndian => array
                .chunks_exact(size_of::<T>())
                .map(|data| T::from_le(data.try_into().unwrap()))
                .collect(),
        }
    }
}
