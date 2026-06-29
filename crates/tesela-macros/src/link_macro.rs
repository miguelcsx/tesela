//! `#[derive(LinkType)]` derive macro implementation.

use crate::helpers::to_snake_case;
use darling::FromAttributes;
use proc_macro::TokenStream;
use quote::quote;
use syn::{DeriveInput, parse_macro_input};

#[derive(Debug, Default, FromAttributes)]
#[darling(attributes(tesela))]
struct LinkArgs {
    from: String,
    to: String,
    #[darling(default)]
    cardinality: Option<String>,
    #[darling(default)]
    display: Option<String>,
}

pub(crate) fn expand(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    let struct_name = &input.ident;
    let struct_name_str = struct_name.to_string();
    let api_name_str = to_snake_case(&struct_name_str);

    let macro_args = match LinkArgs::from_attributes(&input.attrs) {
        Ok(v) => v,
        Err(e) => return TokenStream::from(e.write_errors()),
    };

    let from_str = &macro_args.from;
    let to_str = &macro_args.to;
    let display_name = match macro_args.display {
        Some(value) => value,
        None => struct_name_str,
    };
    let cardinality_str = macro_args
        .cardinality
        .as_deref()
        .map_or("one_to_many", |value| value);

    let cardinality_tokens = match cardinality_str {
        "one_to_one" => quote!(::tesela_core::LinkCardinality::OneToOne),
        "many_to_many" => quote!(::tesela_core::LinkCardinality::ManyToMany),
        _ => quote!(::tesela_core::LinkCardinality::OneToMany),
    };

    let expanded = quote! {
        impl #struct_name {
            /// Return the Tesela `LinkType` definition for this struct.
            pub fn tesela_link_type() -> ::tesela_ir::LinkType {
                ::tesela_ir::LinkType {
                    api_name: ::tesela_core::ApiName::new_unchecked(#api_name_str),
                    display: Some(#display_name.to_string()),
                    from: ::tesela_core::ApiName::new_unchecked(#from_str),
                    to: ::tesela_core::ApiName::new_unchecked(#to_str),
                    cardinality: #cardinality_tokens,
                    source: None,
                    mappings: Vec::new(),
                    junction: None,
                    deprecated_at: None,
                    metadata: None,
                }
            }
        }
    };

    TokenStream::from(expanded)
}
