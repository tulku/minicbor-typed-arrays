use crate::element::ElementType;
use crate::endianness::Endianness;
use minicbor::data::{IanaTag, Tag};
use minicbor::decode::Error;

/// Map a CBOR tag to a typed-array element type and endianness.
///
/// Single-byte element types canonicalize to [`Endianness::Big`]. Returns an
/// error for `f128` typed arrays (unsupported) and for any non-typed-array tag.
pub(crate) fn element_type_from_tag(tag: Tag) -> Result<(ElementType, Endianness), Error> {
    let iana = IanaTag::try_from(tag).map_err(|_| Error::message("not a typed-array tag"))?;
    let result = match iana {
        IanaTag::TypedArrayU8 => (ElementType::U8, Endianness::Big),
        IanaTag::TypedArrayU8Clamped => (ElementType::U8Clamped, Endianness::Big),
        IanaTag::TypedArrayI8 => (ElementType::I8, Endianness::Big),

        IanaTag::TypedArrayU16B => (ElementType::U16, Endianness::Big),
        IanaTag::TypedArrayU16L => (ElementType::U16, Endianness::Little),
        IanaTag::TypedArrayU32B => (ElementType::U32, Endianness::Big),
        IanaTag::TypedArrayU32L => (ElementType::U32, Endianness::Little),
        IanaTag::TypedArrayU64B => (ElementType::U64, Endianness::Big),
        IanaTag::TypedArrayU64L => (ElementType::U64, Endianness::Little),

        IanaTag::TypedArrayI16B => (ElementType::I16, Endianness::Big),
        IanaTag::TypedArrayI16L => (ElementType::I16, Endianness::Little),
        IanaTag::TypedArrayI32B => (ElementType::I32, Endianness::Big),
        IanaTag::TypedArrayI32L => (ElementType::I32, Endianness::Little),
        IanaTag::TypedArrayI64B => (ElementType::I64, Endianness::Big),
        IanaTag::TypedArrayI64L => (ElementType::I64, Endianness::Little),

        #[cfg(feature = "half")]
        IanaTag::TypedArrayF16B => (ElementType::F16, Endianness::Big),
        #[cfg(feature = "half")]
        IanaTag::TypedArrayF16L => (ElementType::F16, Endianness::Little),

        IanaTag::TypedArrayF32B => (ElementType::F32, Endianness::Big),
        IanaTag::TypedArrayF32L => (ElementType::F32, Endianness::Little),
        IanaTag::TypedArrayF64B => (ElementType::F64, Endianness::Big),
        IanaTag::TypedArrayF64L => (ElementType::F64, Endianness::Little),

        IanaTag::TypedArrayF128B | IanaTag::TypedArrayF128L => {
            return Err(Error::message("f128 typed arrays are unsupported"));
        }
        _ => return Err(Error::message("not a typed-array tag")),
    };
    Ok(result)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn check(et: ElementType, end: Endianness, expect_end: Endianness) {
        let tag = et.tag(end).tag();
        assert_eq!(element_type_from_tag(tag).unwrap(), (et, expect_end));
    }

    #[test]
    fn forward_then_reverse_is_identity() {
        for end in [Endianness::Big, Endianness::Little] {
            // multi-byte types preserve endianness
            for et in [
                ElementType::U16,
                ElementType::U32,
                ElementType::U64,
                ElementType::I16,
                ElementType::I32,
                ElementType::I64,
                ElementType::F32,
                ElementType::F64,
            ] {
                check(et, end, end);
            }
            // single-byte types canonicalize to Big
            for et in [ElementType::U8, ElementType::U8Clamped, ElementType::I8] {
                check(et, end, Endianness::Big);
            }
        }
    }

    #[cfg(feature = "half")]
    #[test]
    fn f16_round_trips() {
        for end in [Endianness::Big, Endianness::Little] {
            let tag = ElementType::F16.tag(end).tag();
            assert_eq!(element_type_from_tag(tag).unwrap(), (ElementType::F16, end));
        }
    }

    #[test]
    fn non_typed_array_tag_errors() {
        assert!(element_type_from_tag(Tag::new(0)).is_err()); // DateTime
        assert!(element_type_from_tag(Tag::new(99999)).is_err());
    }
}
