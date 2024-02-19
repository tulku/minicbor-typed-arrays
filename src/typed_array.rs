use crate::typed_array_tag::{typed_array_tag, Endianness, FloatLength, IntegerLength};

#[cfg(feature = "half")]
use half::f16;

use crate::typed_array_tag::ArrayTypeLength;
use std::iter::Iterator;

#[derive(PartialEq, Debug, Clone)]
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

    pub fn len(&self) -> usize {
        match self {
            TypedArray::U8(array) => array.len(),
            TypedArray::U16(array) => array.len(),
            TypedArray::U32(array) => array.len(),
            TypedArray::U64(array) => array.len(),
            TypedArray::I8(array) => array.len(),
            TypedArray::I16(array) => array.len(),
            TypedArray::I32(array) => array.len(),
            TypedArray::I64(array) => array.len(),
            #[cfg(feature = "half")]
            TypedArray::F16(array) => array.len(),
            TypedArray::F32(array) => array.len(),
            TypedArray::F64(array) => array.len(),
        }
    }

    pub fn is_empty(&self) -> bool {
        match self {
            TypedArray::U8(array) => array.is_empty(),
            TypedArray::U16(array) => array.is_empty(),
            TypedArray::U32(array) => array.is_empty(),
            TypedArray::U64(array) => array.is_empty(),
            TypedArray::I8(array) => array.is_empty(),
            TypedArray::I16(array) => array.is_empty(),
            TypedArray::I32(array) => array.is_empty(),
            TypedArray::I64(array) => array.is_empty(),
            #[cfg(feature = "half")]
            TypedArray::F16(array) => array.is_empty(),
            TypedArray::F32(array) => array.is_empty(),
            TypedArray::F64(array) => array.is_empty(),
        }
    }

    pub fn iter(&self) -> TypedArrayIterator {
        TypedArrayIterator {
            array: self,
            index: 0,
        }
    }
}

#[derive(Debug, PartialEq, Copy, Clone)]
pub enum TypedArrayValue {
    U8(u8),
    U16(u16),
    U32(u32),
    U64(u64),
    I8(i8),
    I16(i16),
    I32(i32),
    I64(i64),
    #[cfg(feature = "half")]
    F16(f16),
    F32(f32),
    F64(f64),
}

impl TypedArrayValue {
    pub fn to_f64(self) -> f64 {
        match self {
            TypedArrayValue::U8(v) => v as f64,
            TypedArrayValue::U16(v) => v as f64,
            TypedArrayValue::U32(v) => v as f64,
            TypedArrayValue::U64(v) => v as f64,
            TypedArrayValue::I8(v) => v as f64,
            TypedArrayValue::I16(v) => v as f64,
            TypedArrayValue::I32(v) => v as f64,
            TypedArrayValue::I64(v) => v as f64,
            #[cfg(feature = "half")]
            TypedArrayValue::F16(v) => f64::from(v),
            TypedArrayValue::F32(v) => f64::from(v),
            TypedArrayValue::F64(v) => v,
        }
    }

    pub fn to_i64(self) -> i64 {
        match self {
            TypedArrayValue::U8(v) => v as i64,
            TypedArrayValue::U16(v) => v as i64,
            TypedArrayValue::U32(v) => v as i64,
            TypedArrayValue::U64(v) => v as i64,
            TypedArrayValue::I8(v) => v as i64,
            TypedArrayValue::I16(v) => v as i64,
            TypedArrayValue::I32(v) => v as i64,
            TypedArrayValue::I64(v) => v,
            #[cfg(feature = "half")]
            TypedArrayValue::F16(v) => f64::from(v) as i64,
            TypedArrayValue::F32(v) => v as i64,
            TypedArrayValue::F64(v) => v as i64,
        }
    }
}

pub struct TypedArrayIterator<'a> {
    array: &'a TypedArray,
    index: usize,
}

impl<'a> Iterator for TypedArrayIterator<'a> {
    type Item = TypedArrayValue;

    fn next(&mut self) -> Option<Self::Item> {
        let value = match self.array {
            TypedArray::U8(array) => array.get(self.index).map(|&v| TypedArrayValue::U8(v)),
            TypedArray::U16(array) => array.get(self.index).map(|&v| TypedArrayValue::U16(v)),
            TypedArray::U32(array) => array.get(self.index).map(|&v| TypedArrayValue::U32(v)),
            TypedArray::U64(array) => array.get(self.index).map(|&v| TypedArrayValue::U64(v)),
            TypedArray::I8(array) => array.get(self.index).map(|&v| TypedArrayValue::I8(v)),
            TypedArray::I16(array) => array.get(self.index).map(|&v| TypedArrayValue::I16(v)),
            TypedArray::I32(array) => array.get(self.index).map(|&v| TypedArrayValue::I32(v)),
            TypedArray::I64(array) => array.get(self.index).map(|&v| TypedArrayValue::I64(v)),
            #[cfg(feature = "half")]
            TypedArray::F16(array) => array.get(self.index).map(|&v| TypedArrayValue::F16(v)),
            TypedArray::F32(array) => array.get(self.index).map(|&v| TypedArrayValue::F32(v)),
            TypedArray::F64(array) => array.get(self.index).map(|&v| TypedArrayValue::F64(v)),
        };
        self.index += 1;
        value
    }
}

#[cfg(test)]
mod tests {

    use super::*;

    #[test]
    fn test_typed_array_iter() {
        let array = TypedArray::U8(vec![1, 2, 3, 4]);
        let mut iter = array.iter();
        assert_eq!(iter.next(), Some(TypedArrayValue::U8(1)));
        assert_eq!(iter.next(), Some(TypedArrayValue::U8(2)));
        assert_eq!(iter.next(), Some(TypedArrayValue::U8(3)));
        assert_eq!(iter.next(), Some(TypedArrayValue::U8(4)));
        assert_eq!(iter.next(), None);
    }
}
