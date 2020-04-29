#![feature(proc_macro)]

#[macro_use]
extern crate derive_ocaml;
#[macro_use]
extern crate ocaml;

use derive_ocaml::ocaml_ffi;
use std::ops::Add;

#[derive(Debug, Default, ToValue, FromValue)]
#[ocaml(floats_array)]
pub struct Vec3 {
    x: f32,
    y: f32,
    z: f32,
}

impl Add for Vec3 {
    type Output = Vec3;

    fn add(self, r: Self) -> Self::Output {
        let l = self;
        Vec3 { x: l.x + r.x, y: l.y + r.y, z: l.z + r.z }
    }
}

#[ocaml_ffi]
pub fn rust_add_vecs(l: Vec3, r: Vec3) -> Vec3 {
    l + r
}

#[derive(FromValue, ToValue)]
pub enum Unrolled<Item> {
    Empty,
    One(Item),
    Many(Vec<Item>),
}

#[derive(FromValue)]
#[ocaml(unboxed)]
pub struct Bound(usize);

#[ocaml_ffi]
pub fn rust_sum_vecs(vectors: Unrolled<Vec3>, max_items: Bound) -> Vec3 {
    use self::Unrolled::*;
    match vectors {
        Empty => Default::default(),
        One(vec) => vec,
        Many(vectors) => vectors.into_iter().take(max_items.0).fold(Default::default(), Vec3::add),
    }
}
