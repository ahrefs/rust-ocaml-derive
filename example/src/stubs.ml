type vec3 = {
  x: float;
  y: float;
  z: float;
}

type bound = {
  bound: int;
} [@@unboxed]

type 'item unrolled =
    Empty
  | One of 'item
  | Many of 'item array

external add_vecs : vec3 -> vec3 -> vec3 = "rust_add_vecs"
external sum_vecs : vec3 unrolled -> bound -> vec3 = "rust_sum_vecs"
