#[cfg(feature = "stubs")]
extern crate proc_macro;
extern crate proc_macro2;
extern crate quote;
extern crate syn;

#[cfg(feature = "stubs")]
mod stubs;

#[cfg(feature = "stubs")]
#[proc_macro_attribute]
pub fn ocaml_ffi(
    attribute: proc_macro::TokenStream,
    function: proc_macro::TokenStream,
) -> proc_macro::TokenStream {
    stubs::ocaml(attribute, function)
}

#[cfg(feature = "derive")]
#[macro_use]
extern crate synstructure;

#[cfg(feature = "derive")]
mod derive;

#[cfg(feature = "derive")]
decl_derive!([ToValue, attributes(ocaml)] => derive::tovalue_derive);
#[cfg(feature = "derive")]
decl_derive!([FromValue, attributes(ocaml)] => derive::fromvalue_derive);
