use proc_macro::TokenStream;
use quote::quote;
use syn::{parse_macro_input, Data, DeriveInput, Fields};

#[proc_macro_derive(AsSource, attributes(from))]
pub fn derive_as_source(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    let name = &input.ident;

    let variants = match input.data {
        Data::Enum(data) => data.variants,
        _ => panic!("AsSource can only be derived on Enums"),
    };

    let mut match_arms = Vec::new();
    let mut from_impls = Vec::new();

    for variant in variants.iter() {
        let variant_ident = &variant.ident;

        if let Fields::Unnamed(fields) = &variant.fields {
            if fields.unnamed.len() == 1 {
                let inner_type = &fields.unnamed[0].ty;

                match_arms.push(quote! {
                    Self::#variant_ident(inner) => Some(inner),
                });

                let has_from = variant
                    .attrs
                    .iter()
                    .any(|attr| attr.path().is_ident("from"));
                if has_from {
                    from_impls.push(quote! {
                        impl From<#inner_type> for #name {
                            fn from(err: #inner_type) -> Self {
                                Self::#variant_ident(err)
                            }
                        }
                    });
                }
                continue;
            }
        }

        if let Fields::Unit = &variant.fields {
            // Unit variants (like NoPendingClosures) have NO fields or parentheses
            match_arms.push(quote! {
                Self::#variant_ident => None,
            });
        } else {
            match_arms.push(quote! {
                Self::#variant_ident { .. } => None,
            });
        }
    }

    let expanded = quote! {
        impl as_source::AsSource for #name {
            fn next_source(&self) -> Option<&dyn std::fmt::Debug> {
                match self {
                    #(#match_arms)*
                }
            }
        }

        #(#from_impls)*
    };

    TokenStream::from(expanded)
}
