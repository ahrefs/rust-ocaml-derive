use synstructure;
use proc_macro2;
use syn;

#[derive(Default)]
struct Attrs {
    unboxed: bool,
    floats: bool,
}

fn variant_attrs(attrs: &[syn::Attribute]) -> Attrs {
    fn is_ocaml(path: &syn::Path) -> bool {
        path.segments.len() == 1
            && path
                .segments
                .iter()
                .next()
                .map_or(false, |segment| segment.ident == "ocaml")
    }
    attrs
        .iter()
        .find(|attr| is_ocaml(&attr.path))
        .map_or(Default::default(), |attr| {
            if let Some(syn::Meta::List(ref list)) = attr.interpret_meta() {
                list.nested
                    .iter()
                    .fold(Default::default(), |mut acc, meta| match meta {
                        syn::NestedMeta::Meta(syn::Meta::Word(ref ident)) =>
                                if ident == "unboxed" {
                                    if acc.floats {
                                        panic!("in ocaml attrs a variant cannot be both float array and unboxed")
                                    }
                                    acc.unboxed = true;
                                    acc
                                } else if ident == "floats_array" {
                                    if acc.unboxed {
                                        panic!("in ocaml attrs a variant cannot be both float array and unboxed")
                                    }
                                    acc.floats = true;
                                    acc
                            } else {
                                panic!("unexpected ocaml attribute parameter {}", ident)
                            },
                     _ => panic!("unexpected ocaml attribute parameter"),
                    })
            } else {
                panic!("ocaml attribute must take a list of valid attributes in parentheses")
            }
        })
}

pub fn tovalue_derive(s: synstructure::Structure) -> proc_macro2::TokenStream {
    let mut unit_tag = 0u8;
    let mut non_unit_tag = 0u8;
    let is_record_like = s.variants().len() == 1;
    let body = s.variants().iter().map(|variant| {
        let arity = variant.bindings().len();
        let tag_ref = if arity > 0 {
            &mut non_unit_tag
        } else {
            &mut unit_tag
        };
        let tag = *tag_ref;
        *tag_ref += 1;
        let attrs = variant_attrs(&variant.ast().attrs);
        if (attrs.floats || attrs.unboxed) && !is_record_like {
            panic!("ocaml cannot derive unboxed or float arrays for enums")
        }
        if arity == 0 {
            let init = quote!(value = ocaml::Value::i64(#tag as i64););
            variant.fold(init, |_, _| quote!())
        } else if attrs.floats {
            let mut idx = 0usize;
            let init = quote!(
                value = ocaml::Value::alloc(#arity, ocaml::Tag::DoubleArray);
            );
            variant.fold(init, |acc, b| {
                let i = idx;
                idx += 1;
                quote!(#acc; ocaml::Array::from(value.clone()).set_double(#i, *#b as f64).unwrap();)
            })
        } else if attrs.unboxed {
            if variant.bindings().len() > 1 {
                panic!("ocaml cannot unboxed record with multiple fields")
            }
            variant.each(|field| quote!(value = #field.to_value()))
        } else {
            let mut idx = 0usize;
            let ghost = (0..arity)
                .into_iter()
                .map(|idx| quote!(value.store_field(#idx, ocaml::value::UNIT)));
            let init = quote!(
                value = ocaml::Value::alloc(#arity, ocaml::Tag::new(#tag));
                #(#ghost);*;
                );
            variant.fold(init, |acc, b| {
                let i = idx;
                idx += 1;
                quote!(#acc value.store_field(#i, #b.to_value());)
            })
        }
    });
    s.gen_impl(quote! {
        extern crate ocaml;
        gen impl ocaml::ToValue for @Self {
            fn to_value(&self) -> ocaml::Value {
                unsafe {
                    caml_body!{ | |, <value>, {
                        match *self {
                             #(#body),*
                        }
                     }};
                    return value
                }
            }
        }
    })
}

pub fn fromvalue_derive(s: synstructure::Structure) -> proc_macro2::TokenStream {
    let mut unit_tag = 0u8;
    let mut non_unit_tag = 0u8;
    let is_record_like = s.variants().len() == 1;
    let attrs = if is_record_like {
        variant_attrs(s.variants()[0].ast().attrs)
    } else {
        Attrs::default()
    };
    let body = s.variants().iter().map(|variant| {
        let arity = variant.bindings().len();
        let tag_ref = if arity > 0 {
            &mut non_unit_tag
        } else {
            &mut unit_tag
        };
        let attrs = variant_attrs(&variant.ast().attrs);
        if (attrs.floats || attrs.unboxed) && !is_record_like {
            panic!("ocaml cannot derive unboxed records or float arrays for enums")
        }
        let tag = *tag_ref;
        *tag_ref += 1;
        let is_block = arity != 0;
        if attrs.unboxed {
            if arity > 1 {
                panic!("ocaml cannot derive unboxed records with several fields")
            }
            variant.construct(|_, _| quote!(ocaml::FromValue::from_value(value)))
        } else {
            let construct = variant.construct(|field, idx| {
                if attrs.floats {
                    let ty = &field.ty;
                    quote!(ocaml::Array::from(value.clone()).get_double(#idx).unwrap() as #ty)
                } else {
                    quote!(ocaml::FromValue::from_value(value.field(#idx)))
                }
            });
            quote!((#is_block, #tag) => {
                #construct
            }
            )
        }
    });
    if attrs.unboxed {
        s.gen_impl(quote! {
            extern crate ocaml;
            gen impl ocaml::FromValue for @Self {
                fn from_value(value: ocaml::Value) -> Self {
                    #(#body),*
                }
            }
        })
    } else {
        let tag = if !attrs.floats {
            quote!(match value.tag() {
                ocaml::Tag::Tag(tag) => tag,
                ocaml::Tag::Zero => 0,
                _ => panic!(
                    "ocaml ffi: trying to convert a non structural value to a structure/enum"
                ),
            })
        } else {
            quote!( {
                if value.tag() != ocaml::Tag::DoubleArray {
                    panic!("ocaml ffi: trying to convert a value which is not a double array to an unboxed record")
                };
                0
                })
        };
        s.gen_impl(quote! {
            extern crate ocaml;
            gen impl ocaml::FromValue for @Self {
                fn from_value(value: ocaml::Value) -> Self {
                    let is_block = value.is_block();
                    let tag = if !is_block { value.i32_val() as u8 } else { #tag };
                    match (is_block, tag) {
                        #(#body),*
                        _ => panic!("ocaml ffi: received unknown variant while trying to convert ocaml structure/enum to rust"),
                    }
                }
            }
        })
    }
}

