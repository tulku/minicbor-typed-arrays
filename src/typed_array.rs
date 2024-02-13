use crate::typed_array_tag::{typed_array_tag, Endianness, FloatLength, IntegerLength};

#[cfg(feature = "half")]
use half::f16;

use crate::typed_array_tag::ArrayTypeLength;

#[cfg_attr(test, derive(PartialEq, Debug))]
pub enum TypedArray {
    U8(Vec<u8>),
    U16(Vec<u16>),
    U32(Vec<u32>),
    U64(Vec<u64>),
    I8(Vec<i8>),
    I16(Vec<i16>),
    I32(Vec<i32>),
    I64(Vec<i64>),
    #[cfg(feature = "half")]
    F16(Vec<f16>),
    F32(Vec<f32>),
    F64(Vec<f64>),
}

macro_rules! typed_array_from_iter {
    ($($t:ty, $v:ident),*) => {
        $(
            impl FromIterator<$t> for TypedArray {
                fn from_iter<T: IntoIterator<Item = $t>>(iter: T) -> Self {
                    let mut inner = Vec::new();
                    for i in iter {
                        inner.push(i);
                    }
                    TypedArray::$v(inner)
                }
            }
        )*
    };
}

typed_array_from_iter!(
    u8, U8, u16, U16, u32, U32, u64, U64, i8, I8, i16, I16, i32, I32, i64, I64, f32, F32, f64, F64
);
#[cfg(feature = "half")]
typed_array_from_iter!(f16, F16);

impl TypedArray {
    pub fn tag(&self, endianness: Endianness) -> u64 {
        match self {
            TypedArray::U8(_) => {
                typed_array_tag(ArrayTypeLength::UInt(IntegerLength::I8), endianness)
            }
            TypedArray::U16(_) => {
                typed_array_tag(ArrayTypeLength::UInt(IntegerLength::I16), endianness)
            }
            TypedArray::U32(_) => {
                typed_array_tag(ArrayTypeLength::UInt(IntegerLength::I32), endianness)
            }
            TypedArray::U64(_) => {
                typed_array_tag(ArrayTypeLength::UInt(IntegerLength::I64), endianness)
            }
            TypedArray::I8(_) => {
                typed_array_tag(ArrayTypeLength::SInt(IntegerLength::I8), endianness)
            }
            TypedArray::I16(_) => {
                typed_array_tag(ArrayTypeLength::SInt(IntegerLength::I16), endianness)
            }
            TypedArray::I32(_) => {
                typed_array_tag(ArrayTypeLength::SInt(IntegerLength::I32), endianness)
            }
            TypedArray::I64(_) => {
                typed_array_tag(ArrayTypeLength::SInt(IntegerLength::I64), endianness)
            }
            #[cfg(feature = "half")]
            TypedArray::F16(_) => {
                typed_array_tag(ArrayTypeLength::Float(FloatLength::F16), endianness)
            }
            TypedArray::F32(_) => {
                typed_array_tag(ArrayTypeLength::Float(FloatLength::F32), endianness)
            }
            TypedArray::F64(_) => {
                typed_array_tag(ArrayTypeLength::Float(FloatLength::F64), endianness)
            }
        }
    }
}
