# derive-ocaml - Custom derive and procedural macros for easier ocaml <-> rust FFI

*WARNING* this crate is very experimental

derive-ocaml is based on top of [ocaml-rs](https://github.com/zshipko/ocaml-rs) and adds a custom derive macro for `FromValue` and `ToValue`.
The macro supports structs, enums, and unboxed float records.

On top of that it implements a nightly only procedural macro `ocaml-ffi` to ease the boilerplate of writing stubs functions.


```
#[derive(Debug, Default, ToValue, FromValue)]
#[ocaml(floats_array)]
pub struct Vec3 {
    x: f32,
    y: f32,
    z: f32,
}

#[ocaml_ffi]
pub fn rust_add_vecs(l: Vec3, r: Vec3) -> Vec3 {
    l + r
}
```

see `src/example/src/lib.rs` and `src/example/src/stubs.ml` for example
