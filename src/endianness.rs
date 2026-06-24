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
