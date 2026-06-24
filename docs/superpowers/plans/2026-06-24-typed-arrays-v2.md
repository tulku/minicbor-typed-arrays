# Typed Arrays v2 Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Rewrite the crate's RFC8746 typed-array support against `minicbor` 2.2 and `half` 2.7, replacing the per-type `enum TypedArray { U8(Vec<u8>), ... }` with a single zero-copy `TypedArray<C>` that stores the raw byte payload plus an `ElementType` descriptor and iterates into an `Element` value enum.

**Architecture:** A single macro table is the source of truth for the 12 element kinds; it generates both the value-less `ElementType` and the value-carrying `Element`, plus width / tag / chunk-decode. Tag handling delegates to `minicbor::data::IanaTag` (deleting the hand-rolled `typed_array_tag.rs`). Endianness is a property of the array, so `TypedArrayContext`/`EndiannessAware` are deleted. The crate is `#![no_std]` and bare-metal-first: the borrowed `TypedArray<&[u8]>` path needs no allocator; `alloc`/`std` add the owned `Vec<u8>` backing and `from_slice`.

**Tech Stack:** Rust (edition 2021), `minicbor` 2.2 (`no_std`), `half` 2.7 (`no_std`, optional), `test-case` for table tests, `cargo-llvm-cov` + `cargo-crap` for coverage/CRAP gates.

**Reference spec:** `docs/superpowers/specs/2026-06-24-typed-arrays-v2-design.md`

---

## File Structure

| File | Responsibility |
| --- | --- |
| `Cargo.toml` | deps (`minicbor` 2.2, `half` 2.7 optional), feature graph (`std`→`alloc`→borrowed-only) |
| `src/lib.rs` | `#![cfg_attr(not(test), no_std)]`, `extern crate alloc`, `Encode`/`Decode` impls, re-exports |
| `src/endianness.rs` | `Endianness` enum |
| `src/element.rs` | `define_elements!` macro → `ElementType` + `Element`; `Scalar` trait + impls; width / tag / chunk-decode / `to_f64` / `to_i64` |
| `src/tag.rs` | `element_type_from_tag`: `IanaTag` → `(ElementType, Endianness)` (reverse map + f128/unknown errors) |
| `src/typed_array.rs` | `TypedArray<C>`, `new`, `from_slice`, accessors, `len`/`is_empty`, `Iter`, `IntoIterator`, `InvalidLength` |
| `tests/roundtrip.rs` | integration round-trips (owned + borrowed), iterator, error paths |
| `scripts/ci.sh` | the single reproducible CI entry point |
| `.cargo-crap.toml` | CRAP threshold config |
| `.github/workflows/rust.yml` | installs toolchain + tools, runs `scripts/ci.sh` |
| *(deleted)* `src/typed_array_tag.rs`, `src/typed_array_context.rs` | replaced by `IanaTag` + array-owned endianness |

---

## Task 1: Dependencies and clean slate

Set up the new dependency/feature graph and reset `src/` to a minimal compiling `#![no_std]` skeleton so later tasks build incrementally.

**Files:**
- Modify: `Cargo.toml`
- Modify: `src/lib.rs`
- Create: `src/element.rs`, `src/tag.rs` (empty stubs)
- Modify: `src/endianness.rs`, `src/typed_array.rs` (reset to stubs)
- Delete: `src/typed_array_tag.rs`, `src/typed_array_context.rs`

- [ ] **Step 1: Rewrite `Cargo.toml`**

```toml
[package]
name = "minicbor-typed-arrays"
authors = ["Lucas Chiesa <lucas.chiesa@gmail.com>", "Joaquin de Andres <xcancerberox@gmail.com>"]
description = "RFC8746 typed arrays implementation for minicbor."
version = "0.2.0"
license = "BlueOak-1.0.0"
edition = "2021"

[dependencies]
minicbor = { version = "2.2", default-features = false }
half = { version = "2.7", default-features = false, optional = true }

[dev-dependencies]
test-case = "3"
minicbor = { version = "2.2", features = ["std"] }

[features]
default = ["std", "half"]
std = ["alloc", "minicbor/std"]
alloc = ["minicbor/alloc"]
half = ["dep:half"]
```

- [ ] **Step 2: Delete the obsolete v1 files**

```bash
git rm src/typed_array_tag.rs src/typed_array_context.rs
```

- [ ] **Step 3: Reset `src/endianness.rs` to the new enum**

```rust
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
```

- [ ] **Step 4: Create stub `src/element.rs` and `src/tag.rs`**

`src/element.rs`:

```rust
// Filled in Task 3.
```

`src/tag.rs`:

```rust
// Filled in Task 4.
```

- [ ] **Step 5: Reset `src/typed_array.rs` to an empty stub**

```rust
// Filled in Task 5.
```

- [ ] **Step 6: Rewrite `src/lib.rs` to a compiling skeleton**

```rust
#![cfg_attr(not(test), no_std)]

#[cfg(feature = "alloc")]
extern crate alloc;

mod element;
mod endianness;
mod tag;
mod typed_array;

pub use endianness::Endianness;
```

- [ ] **Step 7: Verify the skeleton compiles on all relevant feature sets**

Run:
```bash
cargo build --all-features
cargo build --no-default-features
cargo build --no-default-features --features half
```
Expected: all three succeed (warnings about unused files are fine; there should be none yet).

- [ ] **Step 8: Commit**

```bash
git add -A
git commit -m "Reset to v2 skeleton: new deps, no_std, drop tag/context modules"
```

---

## Task 2: `Scalar` trait and the element table

Implement the value-conversion trait and the macro that generates `ElementType` and `Element` from one table. This task produces the two core enums plus `width`, `to_f64`, `to_i64`, and `decode_chunk`. (`tag` is added in Task 3 once we map to `IanaTag`.)

**Files:**
- Modify: `src/element.rs`
- Modify: `src/lib.rs` (re-exports)

- [ ] **Step 1: Write the `Scalar` trait and its impls in `src/element.rs`**

```rust
use crate::endianness::Endianness;

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
    fn to_f64(self) -> f64 { f64::from(self) }
    fn to_i64(self) -> i64 { f64::from(self) as i64 }
    #[cfg(feature = "alloc")]
    fn write_be(self, out: &mut alloc::vec::Vec<u8>) {
        out.extend_from_slice(&self.to_be_bytes());
    }
    #[cfg(feature = "alloc")]
    fn write_le(self, out: &mut alloc::vec::Vec<u8>) {
        out.extend_from_slice(&self.to_le_bytes());
    }
}
```

Note: `u8 => ElementType::U8` (not `U8Clamped`); `U8Clamped` has no `Scalar` impl because you only reach it by decoding tag 68 or constructing it explicitly with `TypedArray::new`.

- [ ] **Step 2: Add the `define_elements!` macro and invoke it**

Append to `src/element.rs`:

```rust
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
```

Note: the `$be:path`/`$le:path` tag bindings are matched here but not yet *used* in any expansion — they are consumed by `tag()`, added to this macro in Task 3. A `macro_rules!` metavariable that is matched but unused expands fine and does not warn, so the table compiles as-is in this task. They are written now so the table lives in exactly one place.

- [ ] **Step 3: Re-export the new types from `src/lib.rs`**

Replace the `pub use` block in `src/lib.rs`:

```rust
pub use element::{Element, ElementType, Scalar};
pub use endianness::Endianness;
#[cfg(feature = "half")]
pub use half;
```

- [ ] **Step 4: Write unit tests at the bottom of `src/element.rs`**

```rust
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
```

- [ ] **Step 5: Run the element tests**

Run: `cargo test --all-features element::tests`
Expected: PASS (4 tests + the `f16` test).

- [ ] **Step 6: Verify no-alloc still builds**

Run: `cargo build --no-default-features && cargo build --no-default-features --features half`
Expected: both succeed (the `write_be`/`write_le` methods are `#[cfg(feature = "alloc")]` so absent here).

- [ ] **Step 7: Commit**

```bash
git add -A
git commit -m "Add Scalar trait and element table (ElementType/Element)"
```

---

## Task 3: Forward tag mapping (`ElementType::tag`)

Add the `(ElementType, Endianness) -> IanaTag` method, generated from the same table.

**Files:**
- Modify: `src/element.rs`

- [ ] **Step 1: Add the `tag` method to the macro's `impl ElementType` block**

Inside `define_elements!`, add this method to the `impl ElementType { ... }` block (after `width`):

```rust
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
```

This consumes the `$be`/`$le` bindings declared in the table.

- [ ] **Step 2: Add a unit test for forward tag mapping**

Add to `src/element.rs`'s `tests` module:

```rust
    #[test]
    fn tag_maps_endianness() {
        use minicbor::data::IanaTag;
        assert_eq!(ElementType::U8.tag(Endianness::Little), IanaTag::TypedArrayU8);
        assert_eq!(ElementType::U8.tag(Endianness::Big), IanaTag::TypedArrayU8);
        assert_eq!(ElementType::U32.tag(Endianness::Big), IanaTag::TypedArrayU32B);
        assert_eq!(ElementType::U32.tag(Endianness::Little), IanaTag::TypedArrayU32L);
        assert_eq!(ElementType::F64.tag(Endianness::Big), IanaTag::TypedArrayF64B);
    }
```

- [ ] **Step 3: Run the test**

Run: `cargo test --all-features element::tests::tag_maps_endianness`
Expected: PASS.

- [ ] **Step 4: Commit**

```bash
git add -A
git commit -m "Generate ElementType::tag from the element table"
```

---

## Task 4: Reverse tag mapping (`element_type_from_tag`)

Map an incoming `IanaTag` back to `(ElementType, Endianness)`, returning errors for f128 and non-typed-array tags. Hand-written (the reverse map can't be naively macro-generated because single-byte types share one tag for both endiannesses).

**Files:**
- Modify: `src/tag.rs`
- Modify: `src/lib.rs` (make `tag` module usable internally)

- [ ] **Step 1: Implement `element_type_from_tag` in `src/tag.rs`**

```rust
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
```

Note: without the `half` feature, `IanaTag::TypedArrayF16B`/`F16L` fall through to the `_` arm and error — correct, since we cannot produce an `f16`.

- [ ] **Step 2: Add a round-trip consistency test in `src/tag.rs`**

```rust
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
                ElementType::U16, ElementType::U32, ElementType::U64,
                ElementType::I16, ElementType::I32, ElementType::I64,
                ElementType::F32, ElementType::F64,
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
```

Note on `IanaTag::tag()`: `ElementType::tag` returns an `IanaTag`; calling `.tag()` on it yields the `minicbor::data::Tag`. If the method name differs in this minicbor version, use `Tag::from(iana)` instead (verify against `minicbor::data::IanaTag` docs during Step 3).

- [ ] **Step 3: Run the tag tests**

Run: `cargo test --all-features tag::tests`
Expected: PASS. If a compile error mentions `IanaTag::tag` or `Tag::new`, consult `minicbor::data` docs and adjust to the actual constructor (`Tag::new(u64)` / `IanaTag::tag()` are the expected names) — then re-run.

- [ ] **Step 4: Verify no-default-features build (f16 arms cfg'd out)**

Run: `cargo build --no-default-features`
Expected: success (no reference to `ElementType::F16`).

- [ ] **Step 5: Commit**

```bash
git add -A
git commit -m "Add reverse tag mapping element_type_from_tag"
```

---

## Task 5: The `TypedArray<C>` type

The zero-copy array: construction with length validation + single-byte endianness canonicalization, accessors, lazy iterator, and the `alloc`-only `from_slice` convenience constructor.

**Files:**
- Modify: `src/typed_array.rs`
- Modify: `src/lib.rs` (re-exports)

- [ ] **Step 1: Write the struct, error, constructors, and accessors**

```rust
use crate::element::{Element, ElementType, Scalar};
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
        let endianness = if width == 1 { Endianness::Big } else { endianness };
        Ok(Self { element_type, endianness, bytes })
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
        TypedArray::new(T::ELEMENT_TYPE, endianness, bytes).expect("from_slice produces valid length")
    }
}
```

- [ ] **Step 2: Write the iterator**

Append to `src/typed_array.rs`:

```rust
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
```

- [ ] **Step 3: Re-export from `src/lib.rs`**

Add to `src/lib.rs`:

```rust
pub use typed_array::{InvalidLength, Iter, TypedArray, TypedArrayRef};
#[cfg(feature = "alloc")]
pub use typed_array::OwnedTypedArray;
```

- [ ] **Step 4: Write unit tests at the bottom of `src/typed_array.rs`**

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn new_rejects_misaligned_length() {
        let err = TypedArray::new(ElementType::U16, Endianness::Big, &[0u8, 1, 2][..]).unwrap_err();
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
        let a = TypedArray::new(ElementType::U16, Endianness::Big, &[0x12, 0x34, 0x00, 0x01][..]).unwrap();
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
```

Note: unit tests run under `cfg(test)` where `std` is available, so `alloc::vec!`/`alloc::vec::Vec` resolve. The `#[cfg(feature = "alloc")]` guard on `from_slice` tests keeps `cargo test --no-default-features` honest.

- [ ] **Step 5: Run the tests**

Run: `cargo test --all-features typed_array::tests`
Expected: PASS (5 tests).

- [ ] **Step 6: Verify the no-alloc build still compiles**

Run: `cargo build --no-default-features && cargo build --no-default-features --features half`
Expected: success (`from_slice`, `OwnedTypedArray` cfg'd out; `TypedArrayRef` + `new` + `iter` remain).

- [ ] **Step 7: Commit**

```bash
git add -A
git commit -m "Add TypedArray<C> with new/from_slice/iter and length validation"
```

---

## Task 6: `Encode` and `Decode`

Wire the array into minicbor: encode tag + raw bytes; decode borrows (`&[u8]`) or copies (`Vec<u8>`).

**Files:**
- Modify: `src/lib.rs`
- Create: `tests/roundtrip.rs`

- [ ] **Step 1: Implement `Encode` in `src/lib.rs`**

Add after the module declarations / re-exports:

```rust
use crate::tag::element_type_from_tag;
use crate::typed_array::TypedArray;

impl<C, Ctx> minicbor::Encode<Ctx> for TypedArray<C>
where
    C: AsRef<[u8]>,
{
    fn encode<W: minicbor::encode::Write>(
        &self,
        e: &mut minicbor::Encoder<W>,
        _ctx: &mut Ctx,
    ) -> Result<(), minicbor::encode::Error<W::Error>> {
        let tag = self.element_type().tag(self.endianness());
        e.tag(tag)?.bytes(self.as_bytes())?;
        Ok(())
    }
}
```

Note: `Encoder::tag` takes `impl Into<Tag>`, and `IanaTag: Into<Tag>`, so passing the `IanaTag` directly works. If the compiler rejects it, use `e.tag(tag.tag())?` (the `IanaTag -> Tag` conversion).

- [ ] **Step 2: Implement borrowed `Decode` in `src/lib.rs`**

```rust
impl<'b, Ctx> minicbor::Decode<'b, Ctx> for TypedArray<&'b [u8]> {
    fn decode(
        d: &mut minicbor::Decoder<'b>,
        _ctx: &mut Ctx,
    ) -> Result<Self, minicbor::decode::Error> {
        let tag = d.tag()?;
        let (element_type, endianness) = element_type_from_tag(tag)?;
        let bytes = d.bytes()?;
        TypedArray::new(element_type, endianness, bytes)
            .map_err(|e| minicbor::decode::Error::message("invalid typed array length").with_message(e))
    }
}
```

Note: if `Error::with_message` does not exist in this minicbor version, replace the `.map_err(...)` with:
`.map_err(|_| minicbor::decode::Error::message("typed array byte length is not a multiple of element width"))`.

- [ ] **Step 3: Implement owned `Decode` (alloc) in `src/lib.rs`**

```rust
#[cfg(feature = "alloc")]
impl<'b, Ctx> minicbor::Decode<'b, Ctx> for TypedArray<alloc::vec::Vec<u8>> {
    fn decode(
        d: &mut minicbor::Decoder<'b>,
        _ctx: &mut Ctx,
    ) -> Result<Self, minicbor::decode::Error> {
        let tag = d.tag()?;
        let (element_type, endianness) = element_type_from_tag(tag)?;
        let bytes = d.bytes()?.to_vec();
        TypedArray::new(element_type, endianness, bytes).map_err(|_| {
            minicbor::decode::Error::message(
                "typed array byte length is not a multiple of element width",
            )
        })
    }
}
```

- [ ] **Step 4: Run a quick compile check**

Run: `cargo build --all-features && cargo build --no-default-features`
Expected: both succeed. Fix any `Encoder::tag` / `Error` API mismatches per the notes in Steps 1–2.

- [ ] **Step 5: Write the integration tests `tests/roundtrip.rs`**

```rust
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
    let array = TypedArray::new(ElementType::U8Clamped, Endianness::Big, vec![250u8, 251, 252]).unwrap();
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
    e.tag(IanaTag::TypedArrayF128B).unwrap().bytes(&[0u8; 16]).unwrap();
    let r: Result<OwnedTypedArray, _> = minicbor::decode(&buf);
    assert!(r.is_err());
}
```

- [ ] **Step 6: Run the integration tests**

Run: `cargo test --all-features --test roundtrip`
Expected: PASS (all `roundtrip_*`, `iter_values`, `u8_clamped_round_trips_via_new`, and both `decode_rejects_*`).

- [ ] **Step 7: Run the entire test suite across feature sets**

Run:
```bash
cargo test --all-features
cargo test --no-default-features --features alloc
```
Expected: PASS. (`--no-default-features --features alloc` exercises owned arrays without `half`/`std`-only paths; note unit tests still compile with `std` because of `cfg_attr(not(test), no_std)`.)

- [ ] **Step 8: Commit**

```bash
git add -A
git commit -m "Implement Encode/Decode and add round-trip integration tests"
```

---

## Task 7: CI script, CRAP config, and workflow

Put all checks in one locally-runnable script and have GitHub Actions call it.

**Files:**
- Create: `scripts/ci.sh`
- Create: `.cargo-crap.toml`
- Modify: `.github/workflows/rust.yml`

- [ ] **Step 1: Write `scripts/ci.sh`**

```bash
#!/usr/bin/env bash
# Single source of truth for CI. Run locally with: bash scripts/ci.sh
set -euo pipefail

echo "==> rustfmt"
cargo fmt --all --check

echo "==> clippy (all features)"
cargo clippy --all-targets --all-features -- -D warnings

echo "==> build: feature matrix"
cargo build --all-features
cargo build --no-default-features
cargo build --no-default-features --features half

echo "==> build: bare-metal (thumbv7em-none-eabi, no alloc)"
cargo build --no-default-features --target thumbv7em-none-eabi
cargo build --no-default-features --features half --target thumbv7em-none-eabi

echo "==> test (all features)"
cargo test --all-features

echo "==> coverage (llvm-cov -> lcov)"
cargo llvm-cov --all-features --lcov --output-path lcov.info --fail-under-lines 90

echo "==> CRAP metric"
cargo crap --lcov lcov.info --fail-above

echo "==> CI OK"
```

- [ ] **Step 2: Make it executable**

```bash
chmod +x scripts/ci.sh
```

- [ ] **Step 3: Write `.cargo-crap.toml`**

```toml
# CRAP = complexity^2 * (1 - coverage)^3 + complexity, per function.
# Start at the tool default; tune after the first real run (see plan note).
threshold = 30.0
fail-above = true
```

- [ ] **Step 4: Rewrite `.github/workflows/rust.yml`**

```yaml
name: CI

on:
  push:
    branches: [ "main" ]
  pull_request:
    branches: [ "main" ]

env:
  CARGO_TERM_COLOR: always

jobs:
  ci:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4

      - name: Install Rust toolchain
        run: |
          rustup component add rustfmt clippy llvm-tools
          rustup target add thumbv7em-none-eabi

      - name: Install cargo-llvm-cov
        uses: taiki-e/install-action@v2
        with:
          tool: cargo-llvm-cov

      - name: Install cargo-crap
        run: cargo install cargo-crap --locked

      - name: Run CI checks
        run: bash scripts/ci.sh
```

- [ ] **Step 5: Run the script locally (requires the target + tools installed)**

Run:
```bash
rustup target add thumbv7em-none-eabi
cargo install cargo-llvm-cov cargo-crap --locked
bash scripts/ci.sh
```
Expected: every stage prints its banner and the script ends with `==> CI OK`. If `--fail-under-lines 90` fails, note the actual coverage and either add tests for the uncovered lines or adjust the threshold (record the decision in the commit message). If `cargo crap` flags a function, lower the function's complexity or raise the threshold in `.cargo-crap.toml` with a one-line justification.

- [ ] **Step 6: Commit**

```bash
git add -A
git commit -m "Add reproducible CI script with coverage and CRAP gates"
```

---

## Task 8: README and final verification

Update user-facing docs to the new API and do a final full-matrix pass.

**Files:**
- Create/Modify: `README.md` (if absent, create a short one)

- [ ] **Step 1: Write a `README.md` usage section reflecting the new API**

````markdown
# minicbor-typed-arrays

RFC8746 typed-array support for [`minicbor`](https://crates.io/crates/minicbor).

A `TypedArray<C>` stores the raw RFC8746 byte payload plus its element type and
endianness, and iterates into `Element` values lazily — zero-copy and
`#![no_std]` friendly.

## Features

- `std` (default) → implies `alloc`.
- `alloc` (default via `std`) → owned `Vec<u8>`-backed arrays + `TypedArray::from_slice`.
- `half` (default) → `f16` element support.
- Bare-metal (no allocator): build with `--no-default-features` and use the
  borrowed `TypedArrayRef<'_>` decode path.

## Example

```rust
use minicbor_typed_arrays::{Endianness, OwnedTypedArray, TypedArray, TypedArrayRef};

let array = TypedArray::from_slice::<f32>(&[1.0, 2.0, 3.0], Endianness::Little);
let mut buf = Vec::new();
minicbor::encode(&array, &mut buf).unwrap();

// Owned decode (needs `alloc`):
let owned: OwnedTypedArray = minicbor::decode(&buf).unwrap();
assert_eq!(owned.len(), 3);

// Zero-copy borrowed decode (works without an allocator):
let borrowed: TypedArrayRef = minicbor::decode(&buf).unwrap();
for element in &borrowed {
    println!("{}", element.to_f64());
}
```
````

- [ ] **Step 2: Run the full CI script one final time**

Run: `bash scripts/ci.sh`
Expected: ends with `==> CI OK`.

- [ ] **Step 3: Commit**

```bash
git add -A
git commit -m "Document v2 API in README"
```

---

## Notes for the implementer

- **minicbor 2.x API drift:** The exact names `Encoder::tag(impl Into<Tag>)`, `IanaTag::tag() -> Tag`, `Tag::new(u64)`, `Decoder::tag()`, `Decoder::bytes() -> &'b [u8]`, and `Error::message(&str)` are expected but unverified against 2.2.x at plan-writing time. Each relevant step has a fallback note; if a name is wrong, check `https://docs.rs/minicbor/latest/minicbor/` and adjust — the test that follows will confirm.
- **`half` re-export:** `Element::F16(half::f16)` exposes `half::f16` in the public API, so the crate re-exports `half` (`pub use half;` under the feature). Downstream users construct `f16` via `minicbor_typed_arrays::half::f16` or their own `half` dep.
- **Single-byte canonicalization** (endianness → `Big` for width-1 types) lives in `TypedArray::new`, so both `from_slice` and decode agree and `PartialEq` round-trips hold.
- **CRAP threshold:** `30.0` is the tool default and a placeholder for the first run; tune once real scores exist (per the spec).
