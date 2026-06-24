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

## Development

All CI checks live in one reproducible script:

```bash
bash scripts/ci.sh
```

It runs `rustfmt`, `clippy`, the feature/bare-metal build matrix, the test
suite, `cargo llvm-cov` (line-coverage gate), and `cargo crap` (CRAP metric
gate). Running it locally requires the `thumbv7em-none-eabi` target plus
`cargo-llvm-cov` and `cargo-crap` installed (see `.github/workflows/rust.yml`).
