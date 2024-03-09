use crate::{endianness::{self, ToFromBytes}, typed_array_tag::{typed_array_tag, Endianness, FloatLength, IntegerLength}};

#[cfg(feature = "half")]
use half::f16;

use crate::typed_array_tag::ArrayTypeLength;

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
    pub fn tag(&self, endianness: Endianness) -> u64 {
        match self {
            TypedArrayValue::U8(_) => {
                typed_array_tag(ArrayTypeLength::UInt(IntegerLength::I8), endianness)
            }
            TypedArrayValue::U16(_) => {
                typed_array_tag(ArrayTypeLength::UInt(IntegerLength::I16), endianness)
            }
            TypedArrayValue::U32(_) => {
                typed_array_tag(ArrayTypeLength::UInt(IntegerLength::I32), endianness)
            }
            TypedArrayValue::U64(_) => {
                typed_array_tag(ArrayTypeLength::UInt(IntegerLength::I64), endianness)
            }
            TypedArrayValue::I8(_) => {
                typed_array_tag(ArrayTypeLength::SInt(IntegerLength::I8), endianness)
            }
            TypedArrayValue::I16(_) => {
                typed_array_tag(ArrayTypeLength::SInt(IntegerLength::I16), endianness)
            }
            TypedArrayValue::I32(_) => {
                typed_array_tag(ArrayTypeLength::SInt(IntegerLength::I32), endianness)
            }
            TypedArrayValue::I64(_) => {
                typed_array_tag(ArrayTypeLength::SInt(IntegerLength::I64), endianness)
            }
            #[cfg(feature = "half")]
            TypedArrayValue::F16(_) => {
                typed_array_tag(ArrayTypeLength::Float(FloatLength::F16), endianness)
            }
            TypedArrayValue::F32(_) => {
                typed_array_tag(ArrayTypeLength::Float(FloatLength::F32), endianness)
            }
            TypedArrayValue::F64(_) => {
                typed_array_tag(ArrayTypeLength::Float(FloatLength::F64), endianness)
            }
        }
    }

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

impl ToFromBytes<1> for TypedArrayValue
{
    fn from_le(array: &[u8; C]) -> Self {
        match 
        todo!()
    }

    fn from_be(array: &[u8; C]) -> Self {
        todo!()
    }

    fn to_le(&self) -> [u8; C] {
        todo!()
    }

    fn to_be(&self) -> [u8; C] {
        todo!()
    }
}

pub struct TypedArray{
    array: Vec<TypedArrayValue>
}

struct TypedArrayIterator<'a> {
    counter: usize,
    ta: &'a TypedArray,
}

impl TypedArray {

    fn from_vec(vec: Vec<TypedArrayValue>) -> TypedArray
    {
        Self{array: vec}
    }

    pub fn as_ref(&self) -> &[TypedArrayValue]
    {
        self.array.as_slice()
    }

    fn iter(&self) -> TypedArrayIterator {
        TypedArrayIterator {
            counter: 0,
            ta: self
        }
    }

    pub fn tag(&self, endianness: Endianness) -> u64 {
        self.array[0].tag(endianness)
    }
}

impl<'a> Iterator for TypedArrayIterator<'a> {
    type Item = &'a TypedArrayValue;

    fn next(&mut self) -> Option<Self::Item> {
        let i = self.ta.array.get(self.counter)?;
        self.counter += 1;
        Some(i)
    }
}


#[cfg(test)]
mod tests {

    use super::*;

    #[test]
    fn test_typed_array_iter() {
        let array =TypedArray::from_vec(vec![TypedArrayValue::U8(1), TypedArrayValue::U8(2), TypedArrayValue::U8(3), TypedArrayValue::U8(4)]);
        let mut iter = array.iter();
        assert_eq!(iter.next(), Some(TypedArrayValue::U8(1)).as_ref());
        assert_eq!(iter.next(), Some(TypedArrayValue::U8(2)).as_ref());
        assert_eq!(iter.next(), Some(TypedArrayValue::U8(3)).as_ref());
        assert_eq!(iter.next(), Some(TypedArrayValue::U8(4)).as_ref());
        assert_eq!(iter.next(), None);
    }
}
