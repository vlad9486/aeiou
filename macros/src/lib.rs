// Copyright 2021 Vladislav Melnik
// SPDX-License-Identifier: MIT

// TODO: error handling

#[proc_macro_derive(Effect, attributes(input))]
pub fn derive_effect(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    let syn::DeriveInput { attrs, ident, .. } = syn::parse_macro_input!(input);

    let input_ty = match attrs.iter().find(|a| a.path.is_ident("input")) {
        Some(limit) => limit.parse_args::<syn::Type>().unwrap(),
        None => panic!(),
    };

    let t = quote::quote! {
        impl Effect for #ident {
            type Input = #input_ty;
        }
    };
    t.into()
}

#[proc_macro_derive(Select, attributes(part))]
pub fn derive_composable(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    let syn::DeriveInput { ident, data, .. } = syn::parse_macro_input!(input);

    let it = match data {
        syn::Data::Enum(e) => e.variants.into_iter().filter_map(|v| {
            let ident = v.ident;
            v.attrs
                .into_iter()
                .find(|a| a.path.is_ident("part"))
                .map(|part| (part.parse_args::<syn::Type>().unwrap(), ident))
        }),
        _ => panic!(),
    };
    let (ty, id): (Vec<syn::Type>, Vec<syn::Ident>) = it.unzip();

    let t = quote::quote! {
        #(
        impl Select<#ty> for #ident {
            fn take(output: &aeiou::Context<Self>) -> Option<#ty> {
                match output.take()? {
                    #ident::#id(v) => Some(#ty(v)),
                    _ => None,
                }
            }
        }
        )*
    };
    t.into()
}
