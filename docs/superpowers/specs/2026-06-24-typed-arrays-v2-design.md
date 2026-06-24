# minicbor-typed-arrays v2 — Design

Date: 2026-06-24
Status: Approved (pending final spec review)

## Goal

Rewrite the crate's RFC8746 typed-array support against current dependencies and a
cleaner internal model. The new code does **not** need to be source-compatible with v1;
callers will be migrated separately after the new API lands.

Specifically:

1. Upgrade to the latest `minicbor` (2.2.x).
2. Depend on the latest `half` (2.7.x) directly, decoupled from minicbor.
3. Replace the `enum TypedArray { U8(Vec<u8>), ... }` model (one `Vec<T>` per variant)
   with a single `TypedArray` storing the raw byte payload plus an element-type
   descriptor, iterating into a value enum.
4. Use `minicbor::data::IanaTag` (which now provides the RFC8746 typed-array tags)
   instead of hand-rolled tag bit math.

## Non-goals

- Source compatibility with v1.
- f128 support (no Rust scalar type; decoding an f128 tag is an error).
- Multi-dimensional / homogeneous-array tags (RFC8746 §3) — out of scope for this pass.

## Background: what v1 looks like

- `minicbor = 0.20`, `half = 1.x`, effectively std-only (`std::mem`, `std::iter`, `Vec`).
- `TypedArray` is an 11-variant enum, each variant wrapping a `Vec<T>`. Every method
  (`tag`, `len`, `is_empty`, `iter`, plus `Encode`/`Decode`) is an 11-arm match — the
  same shape duplicated ~6 times.
- `typed_array_tag.rs` is 391 lines of hand-rolled tag bit-twiddling plus an exhaustive
  test block. It contains `panic!`/`todo!()` on malformed input.
- `TypedArrayContext` + `EndiannessAware` exist so the encoder can pick endianness at
  encode time (because the array stores native `Vec<T>` values).

## Target architecture

### Core types

Two parallel enums, **both generated from a single macro table** so they cannot drift:

```rust
// value-less type descriptor — what the array stores about its bytes
pub enum ElementType { U8, U8Clamped, U16, U32, U64, I8, I16, I32, I64, F16, F32, F64 }

// value-carrying — only produced by iteration
pub enum Element { U8(u8), U16(u16), U32(u32), U64(u64),
                   I8(i8), I16(i16), I32(i32), I64(i64),
                   F16(f16), F32(f32), F64(f64) }
```

The single source of truth is a macro invocation listing each element once:

```rust
elements! {
    //  variant     rust ty   BE tag           LE tag
    U8        => u8,   TypedArrayU8,        TypedArrayU8,
    U8Clamped => u8,   TypedArrayU8Clamped, TypedArrayU8Clamped,
    U16       => u16,  TypedArrayU16B,      TypedArrayU16L,
    U32       => u32,  TypedArrayU32B,      TypedArrayU32L,
    U64       => u64,  TypedArrayU64B,      TypedArrayU64L,
    I8        => i8,   TypedArrayI8,        TypedArrayI8,
    I16       => i16,  TypedArrayI16B,      TypedArrayI16L,
    I32       => i32,  TypedArrayI32B,      TypedArrayI32L,
    I64       => i64,  TypedArrayI64B,      TypedArrayI64L,
    F16       => f16,  TypedArrayF16B,      TypedArrayF16L,   // gated on `half`
    F32       => f32,  TypedArrayF32B,      TypedArrayF32L,
    F64       => f64,  TypedArrayF64B,      TypedArrayF64L,
}
```

From this one table the macro generates: both enums, the byte width per element, the
`(ElementType, Endianness) -> IanaTag` map, the `IanaTag -> (ElementType, Endianness)`
map, and the per-element chunk-decode (`&[u8] -> Element`).

Note: `U8`/`U8Clamped`/`I8` are single-byte; their tag is endianness-independent and the
same tag maps back to a fixed `ElementType` (decode picks a canonical `Endianness`, which
is irrelevant for 1-byte iteration). `U8Clamped` is kept distinct so RFC8746 tag 68
round-trips faithfully (v1 did not distinguish it).

### The array

```rust
pub struct TypedArray<C = Vec<u8>> {
    element_type: ElementType,
    endianness:   Endianness,
    bytes:        C,            // C: AsRef<[u8]>
}

pub type TypedArrayRef<'b> = TypedArray<&'b [u8]>;   // borrowed / no-alloc
#[cfg(feature = "alloc")]
pub type OwnedTypedArray  = TypedArray<Vec<u8>>;     // owned
```

The array stores the **raw RFC8746 byte payload as-is**, in the endianness it was tagged
with. Nothing is decoded eagerly. This is the key property that makes it bare-metal
friendly: zero allocation, naturally homogeneous, zero-copy on decode.

### Endianness

`Endianness { Big, Little }` becomes a property of the array (the bytes are already laid
out in that order, and the tag reflects it). Consequence: **`TypedArrayContext` and
`EndiannessAware` are deleted.** `Encode`/`Decode` use minicbor's default context.
The caller chooses endianness at construction time.

### API

```rust
// works everywhere, including no-alloc: wrap pre-laid-out bytes.
// Errors if bytes.len() is not a multiple of the element width.
TypedArray::new(element_type: ElementType, endianness: Endianness, bytes: C) -> Result<Self, Error>;

// alloc-only convenience: lay out native scalar values into bytes.
#[cfg(feature = "alloc")]
TypedArray::from_slice::<T: Scalar>(values: &[T], endianness: Endianness) -> OwnedTypedArray;

fn iter(&self) -> Iter<'_>;          // yields Element, decoding chunks lazily
fn len(&self) -> usize;              // bytes.len() / element_width
fn is_empty(&self) -> bool;
fn element_type(&self) -> ElementType;
fn endianness(&self) -> Endianness;
fn as_bytes(&self) -> &[u8];

impl<'a, C: AsRef<[u8]>> IntoIterator for &'a TypedArray<C> { type Item = Element; ... }
```

`Element` keeps the v1 value helpers:

```rust
impl Element { pub fn to_f64(self) -> f64; pub fn to_i64(self) -> i64; }
```

The only multi-arm match remaining in the crate is the single chunk-decode in
`Iter::next` (where bytes become the correct `Element` variant). The per-method matches
that v1 duplicated across `tag`/`len`/`is_empty`/`iter`/`Encode`/`Decode` all collapse.

### Encode / Decode

```rust
// Encode: tag from (element_type, endianness), then the raw bytes. Default context.
impl<C: AsRef<[u8]>, Ctx> minicbor::Encode<Ctx> for TypedArray<C> {
    // e.tag(iana_tag.tag())?; e.bytes(self.bytes.as_ref())?; ok()
}

// Decode (borrowed, no-alloc): borrow the byte string straight from the input.
impl<'b, Ctx> minicbor::Decode<'b, Ctx> for TypedArray<&'b [u8]> { ... }

// Decode (owned): copy into a Vec. alloc/std only.
#[cfg(feature = "alloc")]
impl<'b, Ctx> minicbor::Decode<'b, Ctx> for TypedArray<Vec<u8>> { ... }
```

All malformed-input paths return a proper `minicbor::decode::Error` (no `panic!`/`todo!()`):
- tag is not a supported typed-array tag → error
- `TypedArrayF128B`/`F128L` → "f128 unsupported" error
- byte-string length not a multiple of the element width → error

## Module layout

| file              | contents                                                            |
| ----------------- | ------------------------------------------------------------------- |
| `lib.rs`          | `#![no_std]`, `extern crate alloc` (gated), `Encode`/`Decode`, re-exports |
| `element.rs`      | the `elements!` macro + generated `ElementType`/`Element`, width, chunk-decode, `Scalar` trait, `to_f64`/`to_i64` |
| `tag.rs`          | `(ElementType, Endianness) <-> IanaTag` (generated from the same table) |
| `endianness.rs`   | `Endianness` enum + small `from_le`/`from_be` byte helpers           |
| `typed_array.rs`  | `TypedArray<C>`, constructors, `Iter`, `len`/`is_empty`, type aliases |

## Dependencies & features

```toml
[dependencies]
minicbor = { version = "2.2", default-features = false }
half     = { version = "2.7", default-features = false, optional = true }

[dev-dependencies]
test-case = "*"
minicbor  = { version = "2.2", features = ["std"] }

[features]
default = ["std", "half"]
std     = ["alloc", "minicbor/std"]
alloc   = ["minicbor/alloc"]   # Vec<u8>-backed owned arrays + from_slice
half    = ["dep:half"]         # f16 element support
```

- `half` 2.7 and `minicbor` 2.2 are both `no_std`. We depend on `half` directly at the
  latest version; `f16`'s `to_le_bytes`/`from_le_bytes` cover our byte handling, so we are
  independent of whatever `half` version minicbor uses internally.
- Bare-metal target: `--no-default-features` (optionally `--features half`). No allocator
  required on this path — only `TypedArray<&[u8]>` and `TypedArray::new` are available; the
  owned `Vec<u8>` backing and `from_slice` are gated behind `alloc`.

## Testing

Kept from v1:
- Round-trip (encode -> decode -> compare) for every element type, both endiannesses.

Added:
- Borrowed / no-alloc decode (`TypedArray<&[u8]>`) round-trips.
- Iterator correctness + `to_f64`/`to_i64`.
- `new()` length validation (odd-length payload rejected).
- Error paths: non-typed-array tag, f128 tag, malformed length.
- `IanaTag` mapping is total over the supported set (a test asserts every `ElementType`
  round-trips through both endianness tags).

## CI — one reproducible script

CI runs **only** `scripts/ci.sh`, so `bash scripts/ci.sh` reproduces CI exactly. The
script runs, in order:

1. `cargo fmt --all --check`
2. `cargo clippy --all-targets --all-features -- -D warnings`
3. Feature-matrix build: `default`, `--no-default-features`,
   `--no-default-features --features half`, `--all-features`, **and a build-only
   `thumbv7em-none-eabi`** target to prove the bare-metal no-alloc path compiles.
4. `cargo test --all-features`
5. `cargo llvm-cov --all-features --lcov --output-path lcov.info`
   (with a `--fail-under-lines` coverage gate)
6. `cargo crap --lcov lcov.info --fail-above` with the threshold in `.cargo-crap.toml`

Supporting files:
- `.cargo-crap.toml` — CRAP threshold (start at default `30`, tune after first run).
- `.github/workflows/rust.yml` — installs `llvm-tools-preview`, `cargo-llvm-cov`,
  `cargo-crap`, and the `thumbv7em-none-eabi` target, then runs `scripts/ci.sh`.

CRAP = `comp(m)^2 * (1 - cov(m))^3 + comp(m)` per function; the gate fails CI when any
function exceeds the threshold (a function that is both complex and poorly covered).

## Decisions made during design

- **Zero-copy raw bytes + lazy `Element` iterator** over an eager `Vec<Element>` — keeps
  the array homogeneous by construction, avoids per-element memory blowup, and is the only
  model that works no-alloc on small microcontrollers.
- **No-alloc, no-`std` bare-metal is a first-class target** (borrowed `&[u8]` backing);
  `alloc` and `std` are additive features.
- **Two enums from one macro table** (Option A) — `ElementType` is the value-less
  descriptor the array must store; `Element` is the value iteration produces; the macro
  keeps them and all their mappings in sync from a single source.
- **Reuse `minicbor::data::IanaTag`** instead of hand-rolled tag math (deletes
  `typed_array_tag.rs`).
- **`ElementType::U8Clamped`** added so RFC8746 tag 68 round-trips faithfully.
- **`new()` returns `Result`; decode errors instead of panicking** on malformed input.
