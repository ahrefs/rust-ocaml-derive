use proc_macro::TokenStream;
use proc_macro2;
use proc_macro2::Ident;
use syn::{self, *};

struct OcamlParam {
    ident: Option<Ident>,
    typ: Type,
}

fn args_to_rust_vars<'a>(args: &'a [OcamlParam]) -> impl 'a + Iterator<Item = proc_macro2::TokenStream> {
    args.iter().filter_map(|arg| {
        match arg.ident {
            None => None,
            Some(ref ident) => {
                let typ = &arg.typ;
                Some(quote!( <#typ as ::ocaml::FromValue>::from_value(#ident)))
            },
        }
    })
}

fn args_names<'a>(args: &'a [OcamlParam]) -> impl 'a + Iterator<Item = proc_macro2::TokenStream> {
    args.iter().filter_map(|arg| arg.ident.as_ref().map(|ident| quote!(#ident)))
}

pub fn ocaml(_attribute: TokenStream, function: TokenStream) -> TokenStream {
    let ItemFn {
        ident,
        block,
        decl,
        attrs,
        vis,
        ..
    } = match syn::parse(function).expect("failed to parse tokens as a function") {
        Item::Fn(item) => item,
        _ => panic!("#[ocaml] can only be applied to functions"),
    };
    match vis {
        syn::Visibility::Public(_) => (),
        _ => panic!("#[ocaml] functions must be public"),
    };
    let FnDecl {
        inputs,
        output,
        variadic,
        generics,
        fn_token,
        ..
    } = { *decl };

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
                FnArg::SelfRef(_) | FnArg::SelfValue(_) => panic!("#[ocaml] can only be applied to plain functions, methods are not supported at argument {}", i),
                FnArg::Inferred(_) => panic!("#[ocaml] does not support inferred types at argument {}"),
                FnArg::Ignored(_) => panic!("#[ocaml] ignored type unsupported"),
                FnArg::Captured(ArgCaptured {
                    pat: syn::Pat::Ident(syn::PatIdent {
                        ref ident,
                        by_ref: None,
                        subpat: None,
                        ..
                    }),
                    ref ty,
                    ..
                }) => {
                    let typ = ty.clone();
                    OcamlParam { ident: Some(ident.clone()), typ }
                },
                FnArg::Captured(ArgCaptured {
                    pat: syn::Pat::Wild(_),
                    ref ty,
                    ..
                }) =>
                { let typ = ty.clone(); OcamlParam { ident: None, typ }},
                FnArg::Captured(_) => panic!("#[ocaml] only supports simple argument patterns at argument {}", i),
            }
        })
        .collect::<Vec<OcamlParam>>();

    let where_clause = &generics.where_clause;
    let (returns, return_ty, rust_ty) = match output {
        rust_ty @ ReturnType::Type(_, _) => (true, quote!( -> ::ocaml::core::mlvalues::Value), quote!(#rust_ty)),
        ReturnType::Default => (false, quote!(), quote!()),
    };
    let params = args_names(&args).collect::<Vec<_>>();
    let rust_code = {
        let inputs = inputs.clone();
        // Generate an internal function so that ? and return properly works
        quote!(
            fn internal( #(#inputs), * ) #rust_ty {
                #block
            }
         )};
    let arguments = args_to_rust_vars(&args);
    let body = if returns {
        quote!(
            #rust_code
            caml_body!{ | #(#params), * |, <return_value>, {
                let rust_val = internal(#(#arguments),*);
                return_value = ::ocaml::ToValue::to_value(&rust_val);
            }
            };
            return ::ocaml::core::mlvalues::Value::from(return_value);
        )
    } else {
        quote!(
            #rust_code
            caml_body!{ | #(#params), * |, @code {
                internal(#(#arguments),*);
            }};
            return
        )
    };
    let inputs = args.iter()
        .map(|arg| {
             match arg.ident {
                 Some(ref ident) => quote!{ mut #ident: ::ocaml::core::mlvalues::Value },
                 None => quote!{ _: ::ocaml::core::mlvalues::Value },
             }
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
