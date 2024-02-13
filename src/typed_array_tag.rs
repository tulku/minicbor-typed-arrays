use minicbor::decode::Error;

const INTEGER: u64 = 0;
const FLOAT: u64 = 1;
const UNSIGNED: u64 = 0;
const SIGNED: u64 = 1;

#[cfg_attr(test, derive(PartialEq, Debug))]
#[derive(Clone, Copy)]
pub enum Endianness {
    BigEndian = 0,
    LittleEndian = 1,
}

#[cfg_attr(test, derive(PartialEq, Debug))]
pub enum FloatLength {
    #[cfg(feature = "half")]
    F16 = 0,
    F32 = 1,
    F64 = 2,
}

#[cfg_attr(test, derive(PartialEq, Debug))]
pub enum IntegerLength {
    I8 = 0,
    I16 = 1,
    I32 = 2,
    I64 = 3,
}

#[cfg_attr(test, derive(PartialEq, Debug))]
pub enum ArrayTypeLength {
    Float(FloatLength),
    UInt(IntegerLength),
    SInt(IntegerLength),
}

pub const fn typed_array_tag(length: ArrayTypeLength, endianness: Endianness) -> u64 {
    let type_dependent = match length {
        ArrayTypeLength::Float(length) => FLOAT << 4 | UNSIGNED << 3 | length as u64,
        ArrayTypeLength::UInt(length) => INTEGER << 4 | UNSIGNED << 3 | length as u64,
        ArrayTypeLength::SInt(length) => INTEGER << 4 | SIGNED << 3 | length as u64,
    };
    let header = 0b010 << 5;
    header | type_dependent | (endianness as u64) << 2
}

fn tag_is_typed_array(tag: u64) -> bool {
    (64..=87).contains(&tag) && (tag >> 5) & 0b111 == 0b010
}

pub fn tag_to_array_type(tag: u64) -> Result<(ArrayTypeLength, Endianness), Error> {
    if !tag_is_typed_array(tag) {
        return Err(Error::message("Not a typed array tag"));
    }
    let endianness = match tag >> 2 & 0b1 {
        0 => Endianness::BigEndian,
        1 => Endianness::LittleEndian,
        _ => panic!("Invalid endianness"),
    };
    let signed = tag >> 3 & 0b1;
    let number_type = tag >> 4 & 0b1;
    let length = tag & 0b11;
    let array_type = match (number_type, signed) {
        (INTEGER, UNSIGNED) => ArrayTypeLength::UInt(match length {
            0 => IntegerLength::I8,
            1 => IntegerLength::I16,
            2 => IntegerLength::I32,
            3 => IntegerLength::I64,
            _ => panic!("Invalid unsigned int length"),
        }),
        (INTEGER, SIGNED) => ArrayTypeLength::SInt(match length {
            0 => IntegerLength::I8,
            1 => IntegerLength::I16,
            2 => IntegerLength::I32,
            3 => IntegerLength::I64,
            _ => panic!("Invalid unsigned length"),
        }),
        (FLOAT, UNSIGNED) => ArrayTypeLength::Float(match length {
            #[cfg(feature = "half")]
            0 => FloatLength::F16,
            1 => FloatLength::F32,
            2 => FloatLength::F64,
            3 => todo!("f128 is not supported"),
            _ => panic!("Invalid float length"),
        }),
        _ => panic!("Invalid type"),
    };
    Ok((array_type, endianness))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_tag_is_typed_array() {
        assert!(!tag_is_typed_array(0));
        assert!(!tag_is_typed_array(10));
        assert!(tag_is_typed_array(65));
        assert!(tag_is_typed_array(87));
        assert!(tag_is_typed_array(64));
    }

    #[test]
    fn test_tag_to_array_type_wrong_type() {
        assert!(tag_to_array_type(0).is_err());
        assert!(tag_to_array_type(10).is_err());
        assert!(tag_to_array_type(65).is_ok());
        assert!(tag_to_array_type(64).is_ok());
    }

    #[test]
    fn test_typed_array_tag() {
        assert_eq!(
            typed_array_tag(
                ArrayTypeLength::Float(FloatLength::F32),
                Endianness::BigEndian
            ),
            0b01010001
        );
        assert_eq!(
            typed_array_tag(
                ArrayTypeLength::Float(FloatLength::F32),
                Endianness::LittleEndian
            ),
            0b01010101
        );
        assert_eq!(
            typed_array_tag(
                ArrayTypeLength::Float(FloatLength::F64),
                Endianness::BigEndian
            ),
            0b01010010
        );
        assert_eq!(
            typed_array_tag(
                ArrayTypeLength::Float(FloatLength::F64),
                Endianness::LittleEndian
            ),
            0b01010110
        );
        assert_eq!(
            typed_array_tag(
                ArrayTypeLength::UInt(IntegerLength::I8),
                Endianness::BigEndian
            ),
            0b01000000
        );
        assert_eq!(
            typed_array_tag(
                ArrayTypeLength::UInt(IntegerLength::I8),
                Endianness::LittleEndian
            ),
            0b01000100
        );
        assert_eq!(
            typed_array_tag(
                ArrayTypeLength::UInt(IntegerLength::I16),
                Endianness::BigEndian
            ),
            0b01000001
        );
        assert_eq!(
            typed_array_tag(
                ArrayTypeLength::UInt(IntegerLength::I16),
                Endianness::LittleEndian
            ),
            0b01000101
        );
        assert_eq!(
            typed_array_tag(
                ArrayTypeLength::UInt(IntegerLength::I32),
                Endianness::BigEndian
            ),
            0b01000010
        );
        assert_eq!(
            typed_array_tag(
                ArrayTypeLength::UInt(IntegerLength::I32),
                Endianness::LittleEndian
            ),
            0b01000110
        );
        assert_eq!(
            typed_array_tag(
                ArrayTypeLength::UInt(IntegerLength::I64),
                Endianness::BigEndian
            ),
            0b01000011
        );
        assert_eq!(
            typed_array_tag(
                ArrayTypeLength::UInt(IntegerLength::I64),
                Endianness::LittleEndian
            ),
            0b01000111
        );
        assert_eq!(
            typed_array_tag(
                ArrayTypeLength::SInt(IntegerLength::I8),
                Endianness::BigEndian
            ),
            0b01001000
        );
        assert_eq!(
            typed_array_tag(
                ArrayTypeLength::SInt(IntegerLength::I8),
                Endianness::LittleEndian
            ),
            0b01001100
        );
        assert_eq!(
            typed_array_tag(
                ArrayTypeLength::SInt(IntegerLength::I16),
                Endianness::BigEndian
            ),
            0b01001001
        );
        assert_eq!(
            typed_array_tag(
                ArrayTypeLength::SInt(IntegerLength::I16),
                Endianness::LittleEndian
            ),
            0b01001101
        );

        assert_eq!(
            typed_array_tag(
                ArrayTypeLength::SInt(IntegerLength::I32),
                Endianness::BigEndian
            ),
            0b01001010
        );
        assert_eq!(
            typed_array_tag(
                ArrayTypeLength::SInt(IntegerLength::I32),
                Endianness::LittleEndian
            ),
            0b01001110
        );

        assert_eq!(
            typed_array_tag(
                ArrayTypeLength::SInt(IntegerLength::I64),
                Endianness::BigEndian
            ),
            0b01001011
        );
        assert_eq!(
            typed_array_tag(
                ArrayTypeLength::SInt(IntegerLength::I64),
                Endianness::LittleEndian
            ),
            0b01001111
        );
    }

    #[cfg(feature = "half")]
    #[test]
    fn test_f16_typed_array_tag() {
        assert_eq!(
            typed_array_tag(
                ArrayTypeLength::Float(FloatLength::F16),
                Endianness::BigEndian
            ),
            0b01010000
        );
        assert_eq!(
            typed_array_tag(
                ArrayTypeLength::Float(FloatLength::F16),
                Endianness::LittleEndian
            ),
            0b01010100
        );
    }

    #[test]
    #[should_panic]
    fn float128_is_not_supported() {
        tag_to_array_type(0b01010011).unwrap();
    }

    #[test]
    fn test_tag_to_array_type() {
        assert_eq!(
            tag_to_array_type(0b01000000).unwrap(),
            (
                ArrayTypeLength::UInt(IntegerLength::I8),
                Endianness::BigEndian
            )
        );
        assert_eq!(
            tag_to_array_type(0b01000001).unwrap(),
            (
                ArrayTypeLength::UInt(IntegerLength::I16),
                Endianness::BigEndian
            )
        );
        assert_eq!(
            tag_to_array_type(0b01000010).unwrap(),
            (
                ArrayTypeLength::UInt(IntegerLength::I32),
                Endianness::BigEndian
            )
        );
        assert_eq!(
            tag_to_array_type(0b01000011).unwrap(),
            (
                ArrayTypeLength::UInt(IntegerLength::I64),
                Endianness::BigEndian
            )
        );
        assert_eq!(
            tag_to_array_type(0b01000100).unwrap(),
            (
                ArrayTypeLength::UInt(IntegerLength::I8),
                Endianness::LittleEndian
            )
        );
        assert_eq!(
            tag_to_array_type(0b01000101).unwrap(),
            (
                ArrayTypeLength::UInt(IntegerLength::I16),
                Endianness::LittleEndian
            )
        );
        assert_eq!(
            tag_to_array_type(0b01000110).unwrap(),
            (
                ArrayTypeLength::UInt(IntegerLength::I32),
                Endianness::LittleEndian
            )
        );
        assert_eq!(
            tag_to_array_type(0b01000111).unwrap(),
            (
                ArrayTypeLength::UInt(IntegerLength::I64),
                Endianness::LittleEndian
            )
        );
        assert_eq!(
            tag_to_array_type(0b01010001).unwrap(),
            (
                ArrayTypeLength::Float(FloatLength::F32),
                Endianness::BigEndian
            )
        );
        assert_eq!(
            tag_to_array_type(0b01010010).unwrap(),
            (
                ArrayTypeLength::Float(FloatLength::F64),
                Endianness::BigEndian
            )
        );
        assert_eq!(
            tag_to_array_type(0b01010101).unwrap(),
            (
                ArrayTypeLength::Float(FloatLength::F32),
                Endianness::LittleEndian
            )
        );
        assert_eq!(
            tag_to_array_type(0b01010110).unwrap(),
            (
                ArrayTypeLength::Float(FloatLength::F64),
                Endianness::LittleEndian
            )
        );
    }

    #[cfg(feature = "half")]
    #[test]
    fn test_tag_to_array_type_f16() {
        assert_eq!(
            tag_to_array_type(0b01010000).unwrap(),
            (
                ArrayTypeLength::Float(FloatLength::F16),
                Endianness::BigEndian
            )
        );
        assert_eq!(
            tag_to_array_type(0b01010100).unwrap(),
            (
                ArrayTypeLength::Float(FloatLength::F16),
                Endianness::LittleEndian
            )
        );
    }
}
