//! `#[derive(TraitDef)]` derive macro implementation.

use crate::helpers::{is_option, rust_type_to_data_type, to_snake_case, type_to_string};
use darling::FromAttributes;
use proc_macro::TokenStream;
use proc_macro2::Span;
use quote::quote;
use syn::{DeriveInput, parse_macro_input};

#[derive(Debug, Default, FromAttributes)]
#[darling(attributes(tesela))]
struct TraitArgs {
    #[darling(default)]
    display: Option<String>,
}

#[derive(Debug, Default, FromAttributes)]
#[darling(attributes(tesela))]
struct TraitFieldArgs {
    #[darling(default)]
    description: Option<String>,
    #[darling(default)]
    nullable: darling::util::Flag,
}

pub(crate) fn expand(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    let struct_name = &input.ident;
    let struct_name_str = struct_name.to_string();
    let api_name_str = to_snake_case(&struct_name_str);

    let macro_args = TraitArgs::from_attributes(&input.attrs).unwrap_or_default();
    let display_name = macro_args.display.unwrap_or(struct_name_str);

    let fields = match &input.data {
        syn::Data::Struct(ds) => match &ds.fields {
            syn::Fields::Named(named) => named.named.iter().collect::<Vec<_>>(),
            _ => {
                return syn::Error::new(
                    Span::call_site(),
                    "TraitDef requires a struct with named fields",
                )
                .to_compile_error()
                .into();
            }
        },
        _ => {
            return syn::Error::new(Span::call_site(), "TraitDef can only be applied to structs")
                .to_compile_error()
                .into();
        }
    };

    let mut property_builders = Vec::new();

    for field in &fields {
        let field_name = field
            .ident
            .as_ref()
            .expect("named struct field")
            .to_string();
        let ty_str = type_to_string(&field.ty);
        let is_opt = is_option(&ty_str);
        let data_type = rust_type_to_data_type(&ty_str);

        let field_args = TraitFieldArgs::from_attributes(&field.attrs).unwrap_or_default();
        let field_description = field_args.description.unwrap_or_default();
        let field_nullable = field_args.nullable.is_present() || is_opt;

        property_builders.push(quote! {
            ::tesela_ir::Property {
                api_name: ::tesela_core::ApiName::new_unchecked(#field_name),
                display: None,
                description: if #field_description.is_empty() { None } else { Some(#field_description.to_string()) },
                data_type: #data_type,
                nullable: if #field_nullable { Some(true) } else { None },
                unique: None,
                indexed: None,
                default: None,
                computed: None,
                source_column: None,
                allowed_values: None,
                sort_order: None,
                tags: Vec::new(),
                markings: Vec::new(),
                encrypted: None,
                quality: Vec::new(),
                metadata: None,
            }
        });
    }

    let expanded = quote! {
        impl #struct_name {
            /// Return the Tesela `Trait` definition for this struct.
            pub fn tesela_trait() -> ::tesela_ir::Trait {
                ::tesela_ir::Trait {
                    api_name: ::tesela_core::ApiName::new_unchecked(#api_name_str),
                    display: Some(#display_name.to_string()),
                    description: None,
                    properties: vec![ #(#property_builders),* ],
                }
            }
        }
    };

    TokenStream::from(expanded)
}
