mod endianness;
mod typed_array;
mod typed_array_context;
mod typed_array_tag;

pub use crate::typed_array::TypedArray;
pub use crate::typed_array::TypedArrayValue;
pub use crate::typed_array_context::TypedArrayContext;
pub use endianness::EndiannessAware;
#[cfg(feature = "half")]
use half::f16;
use minicbor::{self, data::Tag, decode::Error};
pub use typed_array_tag::Endianness;
use typed_array_tag::{tag_to_array_type, ArrayTypeLength, FloatLength, IntegerLength};

impl<C> minicbor::Encode<C> for TypedArray
where
    C: EndiannessAware,
{
    fn encode<W: minicbor::encode::Write>(
        &self,
        e: &mut minicbor::Encoder<W>,
        ctx: &mut C,
    ) -> Result<(), minicbor::encode::Error<W::Error>> {
        let tag = Tag::Unassigned(self.tag(ctx.desired_endianness()));
        let output = match self {
            TypedArray::U8(array) => ctx.to_vec8(array),
            TypedArray::U16(array) => ctx.to_vec8(array),
            TypedArray::U32(array) => ctx.to_vec8(array),
            TypedArray::U64(array) => ctx.to_vec8(array),
            TypedArray::I8(array) => ctx.to_vec8(array),
            TypedArray::I16(array) => ctx.to_vec8(array),
            TypedArray::I32(array) => ctx.to_vec8(array),
            TypedArray::I64(array) => ctx.to_vec8(array),
            #[cfg(feature = "half")]
            TypedArray::F16(array) => ctx.to_vec8(array),
            TypedArray::F32(array) => ctx.to_vec8(array),
            TypedArray::F64(array) => ctx.to_vec8(array),
        };

        e.tag(tag)?.bytes(&output)?.ok()
    }
}

impl<'b, C> minicbor::Decode<'b, C> for TypedArray {
    fn decode(
        d: &mut minicbor::Decoder<'b>,
        _ctx: &mut C,
    ) -> Result<Self, minicbor::decode::Error> {
        let tag = match d.tag()? {
            Tag::Unassigned(number) => Ok(number),
            _ => return Err(Error::message("expected typed array tag")),
        }?;
        let (type_length, endianness) = tag_to_array_type(tag)?;
        let bytes = d.bytes()?;
        let array: TypedArray = match type_length {
            ArrayTypeLength::Float(length) => match length {
                #[cfg(feature = "half")]
                FloatLength::F16 => TypedArray::from_u8s::<2, f16>(bytes, &endianness),
                FloatLength::F32 => TypedArray::from_u8s::<4, f32>(bytes, &endianness),
                FloatLength::F64 => TypedArray::from_u8s::<8, f64>(bytes, &endianness),
            },
            ArrayTypeLength::UInt(length) => match length {
                IntegerLength::I8 => TypedArray::from_u8s::<1, u8>(bytes, &endianness),
                IntegerLength::I16 => TypedArray::from_u8s::<2, u16>(bytes, &endianness),
                IntegerLength::I32 => TypedArray::from_u8s::<4, u32>(bytes, &endianness),
                IntegerLength::I64 => TypedArray::from_u8s::<8, u64>(bytes, &endianness),
            },
            ArrayTypeLength::SInt(length) => match length {
                IntegerLength::I8 => TypedArray::from_u8s::<1, i8>(bytes, &endianness),
                IntegerLength::I16 => TypedArray::from_u8s::<2, i16>(bytes, &endianness),
                IntegerLength::I32 => TypedArray::from_u8s::<4, i32>(bytes, &endianness),
                IntegerLength::I64 => TypedArray::from_u8s::<8, i64>(bytes, &endianness),
            },
        };

        Ok(array)
    }
}

#[cfg(test)]
mod test {
    use super::TypedArrayContext;
    use crate::typed_array_tag::Endianness;
    use crate::TypedArray;
    use half::f16;
    use test_case::test_case;

    #[test_case(TypedArray::F16(vec![f16::from_f32(7.0), f16::from_f32(8.0)]); "f16")]
    #[test_case(TypedArray::F32(vec![1.0, 2.0, 3.0, 4.0, 5.0]); "f32")]
    #[test_case(TypedArray::F64(vec![1.0, 2.0, 3.0, 4.0, 5.0]); "f64")]
    #[test_case(TypedArray::I8(vec![1, 2, 3, 4, 5]); "i8")]
    #[test_case(TypedArray::I16(vec![2_i16.pow(10), -2, 3, -4, 5]); "i16")]
    #[test_case(TypedArray::I32(vec![2_i32.pow(20), -2, 3, -4, 5]); "i32")]
    #[test_case(TypedArray::I64(vec![2_i64.pow(40), -2, 3, -4, 5]); "i64")]
    #[test_case(TypedArray::U8(vec![1, 2, 3, 4, 5]); "u8")]
    #[test_case(TypedArray::U16(vec![2_u16.pow(10), 2, 3, 4, 5]); "u16")]
    #[test_case(TypedArray::U32(vec![2_u32.pow(20), 2, 3, 4, 5]); "u32")]
    #[test_case(TypedArray::U64(vec![2_u64.pow(40), 2, 3, 4, 5]); "u64")]

    fn encode_decode_test(input: TypedArray) {
        let mut le_context = TypedArrayContext::new(Endianness::LittleEndian);
        let mut encoder = minicbor::Encoder::new(Vec::new());

        let encoded = encoder
            .encode_with(&input, &mut le_context)
            .unwrap()
            .writer();
        let decoded: TypedArray = minicbor::decode(encoded).unwrap();
        assert_eq!(decoded, input);

        let mut be_context = TypedArrayContext::new(Endianness::BigEndian);

        let encoded = encoder
            .encode_with(&input, &mut be_context)
            .unwrap()
            .writer();
        let decoded: TypedArray = minicbor::decode(encoded).unwrap();
        assert_eq!(decoded, input);
    }
}
