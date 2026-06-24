use crate::element::{Element, ElementType};
#[cfg(feature = "alloc")]
use crate::element::Scalar;
use crate::endianness::Endianness;
use core::fmt;

/// Error returned by [`TypedArray::new`] when the byte payload length is not a
/// multiple of the element width.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct InvalidLength {
    pub len: usize,
    pub width: usize,
}

impl fmt::Display for InvalidLength {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "byte length {} is not a multiple of element width {}",
            self.len, self.width
        )
    }
}

#[cfg(feature = "std")]
impl std::error::Error for InvalidLength {}

/// An RFC8746 typed array: a homogeneous numeric array stored as its raw byte
/// payload plus an element type and endianness.
///
/// Generic over the byte storage `C`:
/// - [`TypedArrayRef`] (`&[u8]`) borrows the payload — no allocator required.
/// - [`OwnedTypedArray`] (`Vec<u8>`) owns it (requires the `alloc` feature).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TypedArray<C> {
    element_type: ElementType,
    endianness: Endianness,
    bytes: C,
}

/// A typed array borrowing its byte payload (no allocation).
pub type TypedArrayRef<'b> = TypedArray<&'b [u8]>;

/// A typed array owning its byte payload.
#[cfg(feature = "alloc")]
pub type OwnedTypedArray = TypedArray<alloc::vec::Vec<u8>>;

impl<C: AsRef<[u8]>> TypedArray<C> {
    /// Wrap a raw RFC8746 byte payload.
    ///
    /// `bytes.len()` must be a multiple of `element_type.width()`. Single-byte
    /// element types canonicalize `endianness` to [`Endianness::Big`].
    pub fn new(
        element_type: ElementType,
        endianness: Endianness,
        bytes: C,
    ) -> Result<Self, InvalidLength> {
        let width = element_type.width();
        let len = bytes.as_ref().len();
        if len % width != 0 {
            return Err(InvalidLength { len, width });
        }
        let endianness = if width == 1 {
            Endianness::Big
        } else {
            endianness
        };
        Ok(Self {
            element_type,
            endianness,
            bytes,
        })
    }

    pub fn element_type(&self) -> ElementType {
        self.element_type
    }

    pub fn endianness(&self) -> Endianness {
        self.endianness
    }

    /// The raw byte payload.
    pub fn as_bytes(&self) -> &[u8] {
        self.bytes.as_ref()
    }

    /// Number of elements.
    pub fn len(&self) -> usize {
        self.bytes.as_ref().len() / self.element_type.width()
    }

    pub fn is_empty(&self) -> bool {
        self.bytes.as_ref().is_empty()
    }

    /// Iterate the elements, decoding each lazily.
    pub fn iter(&self) -> Iter<'_> {
        Iter {
            element_type: self.element_type,
            endianness: self.endianness,
            bytes: self.bytes.as_ref(),
            pos: 0,
        }
    }
}

#[cfg(feature = "alloc")]
impl TypedArray<alloc::vec::Vec<u8>> {
    /// Build an owned typed array from native scalar values, laying them out in
    /// the requested endianness.
    pub fn from_slice<T: Scalar>(values: &[T], endianness: Endianness) -> Self {
        let mut bytes = alloc::vec::Vec::with_capacity(values.len() * core::mem::size_of::<T>());
        for &v in values {
            match endianness {
                Endianness::Big => v.write_be(&mut bytes),
                Endianness::Little => v.write_le(&mut bytes),
            }
        }
        // Length is always a multiple of the width here, so `new` cannot fail.
        TypedArray::new(T::ELEMENT_TYPE, endianness, bytes)
            .expect("from_slice produces valid length")
    }
}

/// Lazy iterator over a [`TypedArray`]'s elements.
pub struct Iter<'a> {
    element_type: ElementType,
    endianness: Endianness,
    bytes: &'a [u8],
    pos: usize,
}

impl<'a> Iterator for Iter<'a> {
    type Item = Element;

    fn next(&mut self) -> Option<Element> {
        let width = self.element_type.width();
        let end = self.pos.checked_add(width)?;
        if end > self.bytes.len() {
            return None;
        }
        let chunk = &self.bytes[self.pos..end];
        self.pos = end;
        Some(self.element_type.decode_chunk(chunk, self.endianness))
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        let remaining = (self.bytes.len() - self.pos) / self.element_type.width();
        (remaining, Some(remaining))
    }
}

impl ExactSizeIterator for Iter<'_> {}

impl<'a, C: AsRef<[u8]>> IntoIterator for &'a TypedArray<C> {
    type Item = Element;
    type IntoIter = Iter<'a>;

    fn into_iter(self) -> Iter<'a> {
        self.iter()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn new_rejects_misaligned_length() {
        let err =
            TypedArray::new(ElementType::U16, Endianness::Big, &[0u8, 1, 2][..]).unwrap_err();
        assert_eq!(err, InvalidLength { len: 3, width: 2 });
    }

    #[test]
    fn single_byte_endianness_is_canonicalized() {
        let a = TypedArray::new(ElementType::U8, Endianness::Little, &[1u8, 2][..]).unwrap();
        assert_eq!(a.endianness(), Endianness::Big);
    }

    #[test]
    fn len_and_empty() {
        let a = TypedArray::new(ElementType::U32, Endianness::Big, &[0u8; 8][..]).unwrap();
        assert_eq!(a.len(), 2);
        assert!(!a.is_empty());
        let e = TypedArray::new(ElementType::U32, Endianness::Big, &[][..]).unwrap();
        assert!(e.is_empty());
        assert_eq!(e.len(), 0);
    }

    #[test]
    fn iter_decodes_elements() {
        let a = TypedArray::new(ElementType::U16, Endianness::Big, &[0x12, 0x34, 0x00, 0x01][..])
            .unwrap();
        let got: alloc::vec::Vec<Element> = a.iter().collect();
        assert_eq!(got, alloc::vec![Element::U16(0x1234), Element::U16(0x0001)]);
        assert_eq!(a.iter().len(), 2);
    }

    #[cfg(feature = "alloc")]
    #[test]
    fn from_slice_round_trips_through_iter() {
        let a = TypedArray::from_slice::<i32>(&[-1, 2, -3], Endianness::Little);
        assert_eq!(a.element_type(), ElementType::I32);
        let vals: alloc::vec::Vec<i64> = a.iter().map(Element::to_i64).collect();
        assert_eq!(vals, alloc::vec![-1, 2, -3]);
    }
}
