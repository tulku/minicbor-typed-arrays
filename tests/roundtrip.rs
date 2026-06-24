use minicbor::data::{IanaTag, Tag};
use minicbor_typed_arrays::{
    Element, ElementType, Endianness, OwnedTypedArray, Scalar, TypedArray, TypedArrayRef,
};

fn roundtrip<T: Scalar + Copy>(values: &[T]) {
    for end in [Endianness::Big, Endianness::Little] {
        let array = TypedArray::from_slice(values, end);

        let mut buf = Vec::new();
        minicbor::encode(&array, &mut buf).expect("encode");

        let owned: OwnedTypedArray = minicbor::decode(&buf).expect("owned decode");
        assert_eq!(owned, array, "owned round-trip ({end:?})");

        let borrowed: TypedArrayRef = minicbor::decode(&buf).expect("borrowed decode");
        assert_eq!(borrowed.element_type(), array.element_type());
        assert_eq!(borrowed.endianness(), array.endianness());
        assert_eq!(borrowed.as_bytes(), array.as_bytes());

        assert_eq!(array.len(), values.len());
    }
}

#[test]
fn roundtrip_u8() {
    roundtrip::<u8>(&[1, 2, 3, 4, 5]);
}
#[test]
fn roundtrip_u16() {
    roundtrip::<u16>(&[2u16.pow(10), 2, 3, 4, 5]);
}
#[test]
fn roundtrip_u32() {
    roundtrip::<u32>(&[2u32.pow(20), 2, 3]);
}
#[test]
fn roundtrip_u64() {
    roundtrip::<u64>(&[2u64.pow(40), 2, 3]);
}
#[test]
fn roundtrip_i8() {
    roundtrip::<i8>(&[-1, 2, -3]);
}
#[test]
fn roundtrip_i16() {
    roundtrip::<i16>(&[2i16.pow(10), -2, 3, -4]);
}
#[test]
fn roundtrip_i32() {
    roundtrip::<i32>(&[2i32.pow(20), -2, 3]);
}
#[test]
fn roundtrip_i64() {
    roundtrip::<i64>(&[2i64.pow(40), -2, 3]);
}
#[test]
fn roundtrip_f32() {
    roundtrip::<f32>(&[1.0, 2.5, -3.25]);
}
#[test]
fn roundtrip_f64() {
    roundtrip::<f64>(&[1.0, 2.5, -3.25]);
}
#[cfg(feature = "half")]
#[test]
fn roundtrip_f16() {
    // `half` is a normal (not dev) dependency, so reach it through the re-export.
    use minicbor_typed_arrays::half::f16;
    roundtrip::<f16>(&[f16::from_f32(7.0), f16::from_f32(-8.5)]);
}

#[test]
fn iter_values() {
    let a = TypedArray::from_slice::<i16>(&[-1, 2, -3], Endianness::Big);
    let got: Vec<Element> = a.iter().collect();
    assert_eq!(got, vec![Element::I16(-1), Element::I16(2), Element::I16(-3)]);
}

#[test]
fn u8_clamped_round_trips_via_new() {
    let array =
        TypedArray::new(ElementType::U8Clamped, Endianness::Big, vec![250u8, 251, 252]).unwrap();
    let mut buf = Vec::new();
    minicbor::encode(&array, &mut buf).unwrap();
    let decoded: OwnedTypedArray = minicbor::decode(&buf).unwrap();
    assert_eq!(decoded.element_type(), ElementType::U8Clamped);
    assert_eq!(decoded, array);
}

#[test]
fn decode_rejects_non_typed_array_tag() {
    let mut buf = Vec::new();
    let mut e = minicbor::Encoder::new(&mut buf);
    e.tag(Tag::new(0)).unwrap().bytes(&[1, 2, 3, 4]).unwrap();
    let r: Result<OwnedTypedArray, _> = minicbor::decode(&buf);
    assert!(r.is_err());
}

#[test]
fn decode_rejects_f128() {
    let mut buf = Vec::new();
    let mut e = minicbor::Encoder::new(&mut buf);
    e.tag(IanaTag::TypedArrayF128B)
        .unwrap()
        .bytes(&[0u8; 16])
        .unwrap();
    let r: Result<OwnedTypedArray, _> = minicbor::decode(&buf);
    assert!(r.is_err());
}
