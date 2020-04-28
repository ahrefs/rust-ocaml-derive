use proc_macro::TokenStream;
use proc_macro2;
use proc_macro2::Ident;
use syn::{self, *};

struct OcamlParam {
    ident: Option<Ident>,
    typ: Type,
}

struct OcamlFFIAttrs {
    ffi_exn: Option<String>,
}

fn args_to_rust_vars<'a>(
    args: &'a [OcamlParam],
) -> impl 'a + Iterator<Item = proc_macro2::TokenStream> {
    args.iter().filter_map(|arg| match arg.ident {
        None => None,
        Some(ref ident) => {
            let typ = &arg.typ;
            Some(quote!( <#typ as ::ocaml::FromValue>::from_value(#ident)))
        }
    })
}

fn args_names<'a>(args: &'a [OcamlParam]) -> impl 'a + Iterator<Item = proc_macro2::TokenStream> {
    args.iter()
        .filter_map(|arg| arg.ident.as_ref().map(|ident| quote!(#ident)))
}

fn parse_attr_args(args: &[NestedMeta]) -> OcamlFFIAttrs {
    let mut ffi_exn = None;
    for arg in args {
        match arg {
            NestedMeta::Meta(Meta::NameValue(MetaNameValue {
                path,
                lit: Lit::Str(value),
                ..
            })) if path.is_ident("ffi_exn") => ffi_exn = Some(value.value()),
            _ => panic!("unrecognized argument {:?}", arg),
        }
    }
    OcamlFFIAttrs { ffi_exn }
}

pub fn ocaml(attribute: TokenStream, function: TokenStream) -> TokenStream {
    let ItemFn {
        attrs,
        block,
        sig,
        vis,
        ..
    } = parse_macro_input!(function as ItemFn);
    let attr_args = parse_macro_input!(attribute as AttributeArgs);
    let ffi_attrs = parse_attr_args(&attr_args);
    match vis {
        syn::Visibility::Public(_) => (),
        _ => panic!("#[ocaml] functions must be public"),
    };
    let Signature {
        ident,
        inputs,
        output,
        variadic,
        generics,
        fn_token,
        ..
    } = sig;

    if !generics.params.is_empty() {
        panic!("#[ocaml] functions must not have generics")
    }
    if variadic.is_some() {
        panic!("#[ocaml] functions must not be variadic")
    }

    let args = inputs
        .iter()
        .enumerate()
        .map(|(i, input)| {
            match *input {
                FnArg::Receiver(_) => panic!("#[ocaml] can only be applied to plain functions, methods are not supported at argument {}", i),
                FnArg::Typed(ref arg) => {
                    if let Type::Infer(_) = *arg.ty {
                        panic!("#[ocaml] does not support inferred types at argument {}")
                    }
                    match *arg.pat {
                        Pat::Wild(_) => OcamlParam { ident: None, typ: Type::clone(&*arg.ty) },
                        Pat::Ident(PatIdent { ref ident, by_ref: None, subpat: None, .. }) => OcamlParam { ident: Some(ident.clone()), typ: Type::clone(&*arg.ty) },
                        _ => panic!("#[ocaml] only supports simple argument patterns at argument {:?}", arg),
                    }
                }
            }
        })
        .collect::<Vec<OcamlParam>>();

    let where_clause = &generics.where_clause;
    let (returns, return_ty, rust_ty) = match output {
        rust_ty @ ReturnType::Type(_, _) => (
            true,
            quote!( -> ::ocaml::core::mlvalues::Value),
            quote!(#rust_ty),
        ),
        ReturnType::Default => (false, quote!(), quote!()),
    };
    let params = args_names(&args).collect::<Vec<_>>();
    let rust_code = {
        let inputs = inputs.iter();
        // Generate an internal function so that ? and return properly works
        quote!(
            #[inline(always)]
            fn internal( #(#inputs),* ) #rust_ty {
                #block
            }
        )
    };
    let arguments = args_to_rust_vars(&args);
    let ffi_exn_id = if let Some(ffi_exn) = &ffi_attrs.ffi_exn {
        let ffi_exn = syn::LitStr::new(ffi_exn, proc_macro2::Span::call_site());
        quote!(
            ::std::option::Option::Some(#ffi_exn.to_string())
        )
    } else {
        quote!(::std::option::Option::None)
    };
    let body = if returns {
        quote!(
            #rust_code
            ::ocaml::caml_body!{ | #(#params), * |, <return_value>, {
                let rust_val = ::std::panic::catch_unwind(::std::panic::AssertUnwindSafe(move || { internal(#(#arguments),*) }));
                match rust_val {
                    Ok(rust_val) =>
                        return_value = ::ocaml::ToValue::to_value(&rust_val),
                    Err(_) => {
                        if let Some(exn_tag) = #ffi_exn_id.and_then(|id: ::std::string::String| ::ocaml::named_value(id)) {
                            ::ocaml::runtime::raise_with_string(&exn_tag, "rust ffi panic")
                        } else {
                            let exn_val =
                                ::ocaml::Value::from(::ocaml::Str::from("no rust ffi exception is registered"));
                            ::ocaml::runtime::invalid_argument_value(
                                &exn_val)
                        }
                    }
                }
            }};
            ::ocaml::core::mlvalues::Value::from(return_value)
        )
    } else {
        quote!(
            #rust_code
            ::ocaml::caml_body!{ | #(#params), * |, @code {
                let rust_val = ::std::panic::catch_unwind(::std::panic::AssertUnwindSafe(move || { internal(#(#arguments),*) }));
                if let Err(_) = rust_val {
                    if let Some(exn_tag) = #ffi_exn_id.and_then(|id: ::std::string::String| ::ocaml::named_value(id)) {
                        ::ocaml::runtime::raise_with_string(&exn_tag, "rust ffi panic")
                    } else {
                        let exn_val =
                            ::ocaml::Value::from(::ocaml::Str::from("no rust ffi exception is registered"));
                        ::ocaml::runtime::invalid_argument_value(
                            &exn_val)
                    }
                }
            }};
        )
    };
    let inputs = args.iter().map(|arg| match arg.ident {
        Some(ref ident) => quote! { mut #ident: ::ocaml::core::mlvalues::Value },
        None => quote! { _: ::ocaml::core::mlvalues::Value },
    });

    let output = quote! {
        #[no_mangle]
        #[allow(unused_mut)]
        #(#attrs)*
        pub unsafe extern #fn_token #ident (#(#inputs),*) #return_ty #where_clause {
            #body
        }
    };
    output.into()
}
