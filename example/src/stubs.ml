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

(* if you want to intercept ffi with custom ocaml exception,
   make sure the custom exception is registered like the following,
   and annotate the ffi function with `ffi_exn = "<exception name>"`
*)
exception RustFFI of string
let _ = Callback.register_exception "rust-ffi" (RustFFI "ffi panic")

external add_vecs : vec3 -> vec3 -> vec3 = "rust_add_vecs"
external sum_vecs : vec3 unrolled -> bound -> vec3 = "rust_sum_vecs"
external test_panic : unit -> unit = "test_panic"
