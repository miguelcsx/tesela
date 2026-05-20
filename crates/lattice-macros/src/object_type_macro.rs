//! `#[derive(ObjectType)]` derive macro implementation.

use crate::helpers::{is_option, rust_type_to_data_type, to_snake_case, type_to_string};
use darling::FromAttributes;
use proc_macro::TokenStream;
use proc_macro2::Span;
use quote::quote;
use syn::{parse_macro_input, DeriveInput};

#[derive(Debug, Default, FromAttributes)]
#[darling(attributes(lattice))]
struct ObjectTypeArgs {
    #[darling(default)]
    datasource: Option<String>,
    #[darling(default)]
    primary_key: Option<String>,
    #[darling(default)]
    display: Option<String>,
}

#[derive(Debug, Default, FromAttributes)]
#[darling(attributes(lattice))]
struct FieldArgs {
    #[darling(default)]
    indexed: darling::util::Flag,
    #[darling(default)]
    unique: darling::util::Flag,
    #[darling(default)]
    nullable: darling::util::Flag,
    #[darling(default)]
    description: Option<String>,
    #[darling(default)]
    source_column: Option<String>,
    #[darling(default)]
    encrypted: darling::util::Flag,
}

pub(crate) fn expand(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    let struct_name = &input.ident;
    let struct_name_str = struct_name.to_string();
    let api_name_str = to_snake_case(&struct_name_str);

    let macro_args = ObjectTypeArgs::from_attributes(&input.attrs).unwrap_or_default();

    let datasource = macro_args
        .datasource
        .as_deref()
        .unwrap_or("memory")
        .to_string();
    let primary_key = macro_args
        .primary_key
        .as_deref()
        .unwrap_or("id")
        .to_string();
    let display_name = macro_args.display.unwrap_or(struct_name_str);

    let fields = match &input.data {
        syn::Data::Struct(ds) => match &ds.fields {
            syn::Fields::Named(named) => named.named.iter().collect::<Vec<_>>(),
            _ => {
                return syn::Error::new(
                    Span::call_site(),
                    "ObjectType requires a struct with named fields",
                )
                .to_compile_error()
                .into();
            }
        },
        _ => {
            return syn::Error::new(
                Span::call_site(),
                "ObjectType can only be applied to structs",
            )
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

        let field_args = FieldArgs::from_attributes(&field.attrs).unwrap_or_default();

        let field_indexed = field_args.indexed.is_present();
        let field_unique = field_args.unique.is_present();
        let field_encrypted = field_args.encrypted.is_present();
        let field_description = field_args.description.unwrap_or_default();
        let field_source_column = field_args.source_column.unwrap_or_default();

        // nullable: explicit flag > Option<T> inference
        let field_nullable = if field_args.nullable.is_present() {
            true
        } else {
            is_opt
        };

        property_builders.push(quote! {
            ::lattice_ir::Property {
                api_name: ::lattice_core::ApiName::new_unchecked(#field_name),
                display: None,
                description: if #field_description.is_empty() { None } else { Some(#field_description.to_string()) },
                data_type: #data_type,
                nullable: if #field_nullable { Some(true) } else { None },
                unique: if #field_unique { Some(true) } else { None },
                indexed: if #field_indexed { Some(true) } else { None },
                default: None,
                computed: None,
                source_column: if #field_source_column.is_empty() { None } else { Some(#field_source_column.to_string()) },
                allowed_values: None,
                sort_order: None,
                tags: Vec::new(),
                markings: Vec::new(),
                encrypted: if #field_encrypted { Some(true) } else { None },
                quality: Vec::new(),
                metadata: None,
            }
        });
    }

    let expanded = quote! {
        impl #struct_name {
            /// Return the Lattice `ObjectType` definition for this struct.
            pub fn lattice_object_type() -> ::lattice_ir::ObjectType {
                ::lattice_ir::ObjectType {
                    api_name: ::lattice_core::ApiName::new_unchecked(#api_name_str),
                    display: Some(#display_name.to_string()),
                    description: None,
                    source: ::lattice_ir::ObjectSource {
                        datasource: ::lattice_core::ApiName::new_unchecked(#datasource),
                        resource: Some(#api_name_str.to_string()),
                    },
                    primary_key: ::lattice_core::ApiName::new_unchecked(#primary_key),
                    properties: vec![ #(#property_builders),* ],
                    traits: Vec::new(),
                    tags: Vec::new(),
                    metadata: None,
                    indexes: Vec::new(),
                    temporal: None,
                    lifecycle: None,
                    scoring: None,
                    classification: None,
                    quality_rules: Vec::new(),
                    lineage: Vec::new(),
                    deprecated_at: None,
                }
            }
        }
    };

    TokenStream::from(expanded)
}
