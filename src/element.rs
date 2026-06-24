/// Byte order of a typed array's elements.
///
/// For single-byte element types (`U8`, `U8Clamped`, `I8`) endianness is
/// meaningless; [`crate::TypedArray::new`] canonicalizes those to
/// [`Endianness::Big`].
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Endianness {
    Big,
    Little,
}

/// A numeric scalar that can back a typed array element.
///
/// Implemented for every primitive RFC8746 element type plus [`half::f16`]
/// (under the `half` feature). Public so that [`crate::TypedArray::from_slice`]
/// can be generic over it.
pub trait Scalar: Copy {
    /// The [`ElementType`] discriminant for this scalar.
    const ELEMENT_TYPE: ElementType;
    fn to_f64(self) -> f64;
    fn to_i64(self) -> i64;
    #[cfg(feature = "alloc")]
    fn write_be(self, out: &mut alloc::vec::Vec<u8>);
    #[cfg(feature = "alloc")]
    fn write_le(self, out: &mut alloc::vec::Vec<u8>);
}

macro_rules! impl_scalar {
    ( $( $ty:ty => $et:expr ),+ $(,)? ) => {
        $(
            impl Scalar for $ty {
                const ELEMENT_TYPE: ElementType = $et;
                fn to_f64(self) -> f64 { self as f64 }
                fn to_i64(self) -> i64 { self as i64 }
                #[cfg(feature = "alloc")]
                fn write_be(self, out: &mut alloc::vec::Vec<u8>) {
                    out.extend_from_slice(&self.to_be_bytes());
                }
                #[cfg(feature = "alloc")]
                fn write_le(self, out: &mut alloc::vec::Vec<u8>) {
                    out.extend_from_slice(&self.to_le_bytes());
                }
            }
        )+
    };
}

impl_scalar!(
    u8  => ElementType::U8,
    u16 => ElementType::U16,
    u32 => ElementType::U32,
    u64 => ElementType::U64,
    i8  => ElementType::I8,
    i16 => ElementType::I16,
    i32 => ElementType::I32,
    i64 => ElementType::I64,
    f32 => ElementType::F32,
    f64 => ElementType::F64,
);

#[cfg(feature = "half")]
impl Scalar for half::f16 {
    const ELEMENT_TYPE: ElementType = ElementType::F16;
    fn to_f64(self) -> f64 {
        f64::from(self)
    }
    fn to_i64(self) -> i64 {
        f64::from(self) as i64
    }
    #[cfg(feature = "alloc")]
    fn write_be(self, out: &mut alloc::vec::Vec<u8>) {
        out.extend_from_slice(&self.to_be_bytes());
    }
    #[cfg(feature = "alloc")]
    fn write_le(self, out: &mut alloc::vec::Vec<u8>) {
        out.extend_from_slice(&self.to_le_bytes());
    }
}

macro_rules! define_elements {
    (
        $(
            $(#[$meta:meta])*
            $variant:ident => $ty:ty, $be:path, $le:path
        );+ $(;)?
    ) => {
        /// The element type of a typed array (value-less descriptor).
        #[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
        pub enum ElementType {
            $( $(#[$meta])* $variant, )+
        }

        /// A single decoded typed-array element.
        #[derive(Clone, Copy, Debug, PartialEq)]
        pub enum Element {
            $( $(#[$meta])* $variant($ty), )+
        }

        impl ElementType {
            /// Width, in bytes, of one element of this type.
            pub const fn width(self) -> usize {
                match self {
                    $( $(#[$meta])* ElementType::$variant => core::mem::size_of::<$ty>(), )+
                }
            }

            /// The IANA tag for this element type in the given endianness.
            ///
            /// Single-byte types ignore `endianness` (both map to the same tag).
            pub fn tag(self, endianness: Endianness) -> minicbor::data::IanaTag {
                match (self, endianness) {
                    $(
                        $(#[$meta])*
                        (ElementType::$variant, Endianness::Big) => $be,
                        $(#[$meta])*
                        (ElementType::$variant, Endianness::Little) => $le,
                    )+
                }
            }

            /// Decode one width-sized chunk into an [`Element`].
            ///
            /// `chunk.len()` must equal `self.width()`.
            pub(crate) fn decode_chunk(self, chunk: &[u8], endianness: Endianness) -> Element {
                match self {
                    $(
                        $(#[$meta])*
                        ElementType::$variant => {
                            let arr = chunk
                                .try_into()
                                .expect("chunk length must equal element width");
                            let value = match endianness {
                                Endianness::Big => <$ty>::from_be_bytes(arr),
                                Endianness::Little => <$ty>::from_le_bytes(arr),
                            };
                            Element::$variant(value)
                        }
                    )+
                }
            }
        }

        impl Element {
            /// Lossy conversion of this element's value to `f64`.
            pub fn to_f64(self) -> f64 {
                match self {
                    $( $(#[$meta])* Element::$variant(v) => Scalar::to_f64(v), )+
                }
            }

            /// Lossy conversion of this element's value to `i64`.
            pub fn to_i64(self) -> i64 {
                match self {
                    $( $(#[$meta])* Element::$variant(v) => Scalar::to_i64(v), )+
                }
            }
        }
    };
}

define_elements! {
    U8        => u8,         minicbor::data::IanaTag::TypedArrayU8,        minicbor::data::IanaTag::TypedArrayU8;
    U8Clamped => u8,         minicbor::data::IanaTag::TypedArrayU8Clamped, minicbor::data::IanaTag::TypedArrayU8Clamped;
    U16       => u16,        minicbor::data::IanaTag::TypedArrayU16B,      minicbor::data::IanaTag::TypedArrayU16L;
    U32       => u32,        minicbor::data::IanaTag::TypedArrayU32B,      minicbor::data::IanaTag::TypedArrayU32L;
    U64       => u64,        minicbor::data::IanaTag::TypedArrayU64B,      minicbor::data::IanaTag::TypedArrayU64L;
    I8        => i8,         minicbor::data::IanaTag::TypedArrayI8,        minicbor::data::IanaTag::TypedArrayI8;
    I16       => i16,        minicbor::data::IanaTag::TypedArrayI16B,      minicbor::data::IanaTag::TypedArrayI16L;
    I32       => i32,        minicbor::data::IanaTag::TypedArrayI32B,      minicbor::data::IanaTag::TypedArrayI32L;
    I64       => i64,        minicbor::data::IanaTag::TypedArrayI64B,      minicbor::data::IanaTag::TypedArrayI64L;
    #[cfg(feature = "half")]
    F16       => half::f16,  minicbor::data::IanaTag::TypedArrayF16B,      minicbor::data::IanaTag::TypedArrayF16L;
    F32       => f32,        minicbor::data::IanaTag::TypedArrayF32B,      minicbor::data::IanaTag::TypedArrayF32L;
    F64       => f64,        minicbor::data::IanaTag::TypedArrayF64B,      minicbor::data::IanaTag::TypedArrayF64L;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn widths_match_scalar_sizes() {
        assert_eq!(ElementType::U8.width(), 1);
        assert_eq!(ElementType::U8Clamped.width(), 1);
        assert_eq!(ElementType::U16.width(), 2);
        assert_eq!(ElementType::U32.width(), 4);
        assert_eq!(ElementType::U64.width(), 8);
        assert_eq!(ElementType::I64.width(), 8);
        assert_eq!(ElementType::F32.width(), 4);
        assert_eq!(ElementType::F64.width(), 8);
    }

    #[test]
    fn decode_chunk_respects_endianness() {
        let be = ElementType::U16.decode_chunk(&[0x12, 0x34], Endianness::Big);
        let le = ElementType::U16.decode_chunk(&[0x12, 0x34], Endianness::Little);
        assert_eq!(be, Element::U16(0x1234));
        assert_eq!(le, Element::U16(0x3412));
    }

    #[test]
    fn tag_maps_endianness() {
        use minicbor::data::IanaTag;
        assert_eq!(
            ElementType::U8.tag(Endianness::Little),
            IanaTag::TypedArrayU8
        );
        assert_eq!(ElementType::U8.tag(Endianness::Big), IanaTag::TypedArrayU8);
        assert_eq!(
            ElementType::U32.tag(Endianness::Big),
            IanaTag::TypedArrayU32B
        );
        assert_eq!(
            ElementType::U32.tag(Endianness::Little),
            IanaTag::TypedArrayU32L
        );
        assert_eq!(
            ElementType::F64.tag(Endianness::Big),
            IanaTag::TypedArrayF64B
        );
    }

    #[test]
    fn element_value_conversions() {
        assert_eq!(Element::I16(-5).to_i64(), -5);
        assert_eq!(Element::U8Clamped(200).to_f64(), 200.0);
        assert_eq!(Element::F64(1.5).to_f64(), 1.5);
        assert_eq!(Element::F32(2.0).to_i64(), 2);
    }

    #[cfg(feature = "half")]
    #[test]
    fn f16_conversions() {
        let v = half::f16::from_f32(3.5);
        assert_eq!(Element::F16(v).to_f64(), 3.5);
        let decoded = ElementType::F16.decode_chunk(&v.to_le_bytes(), Endianness::Little);
        assert_eq!(decoded, Element::F16(v));
    }
}
