use crate::{
    endianness::{EndiannessAware, ToFromBytes},
    typed_array_tag::Endianness,
};

pub struct TypedArrayContext {
    endianness: Endianness,
}

impl TypedArrayContext {
    pub fn new(endianness: Endianness) -> Self {
        Self { endianness }
    }
}

impl EndiannessAware for TypedArrayContext {
    fn desired_endianness(&self) -> Endianness {
        self.endianness
    }

    fn to_vec8<const C: usize, T: ToFromBytes<C>>(&self, array: &[T]) -> Vec<u8> {
        match self.endianness {
            Endianness::BigEndian => array.iter().flat_map(|x| x.to_be()).collect::<Vec<u8>>(),
            Endianness::LittleEndian => array.iter().flat_map(|x| x.to_le()).collect::<Vec<u8>>(),
        }
    }
}
