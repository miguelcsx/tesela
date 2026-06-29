//! `#[derive(ObjectType)]` derive macro implementation.

use crate::helpers::{is_option, rust_type_to_data_type, to_snake_case, type_to_string};
use proc_macro::TokenStream;
use proc_macro2::{Span, TokenStream as TokenStream2};
use quote::quote;
use syn::spanned::Spanned;
use syn::{Attribute, DeriveInput, Expr, LitStr, Result, parse_macro_input};

#[derive(Debug, Default)]
struct ObjectTypeArgs {
    api_name: Option<Expr>,
    datasource: Option<Expr>,
    primary_key: Option<Expr>,
    display: Option<LitStr>,
    description: Option<LitStr>,
}

#[derive(Debug, Default)]
struct FieldArgs {
    indexed: bool,
    unique: bool,
    nullable: bool,
    data_type: Option<Expr>,
    description: Option<LitStr>,
    source_column: Option<LitStr>,
    encrypted: bool,
}

pub(crate) fn expand(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    match expand_checked(input) {
        Ok(tokens) => tokens.into(),
        Err(error) => error.to_compile_error().into(),
    }
}

fn expand_checked(input: DeriveInput) -> Result<TokenStream2> {
    let struct_name = &input.ident;
    let struct_name_str = struct_name.to_string();
    let fallback_api_name = to_snake_case(&struct_name_str);
    let object_args = parse_object_args(&input.attrs)?;

    let datasource = required_expr(
        object_args.datasource,
        struct_name.span(),
        "ObjectType requires #[tesela(datasource = ...)]",
    )?;
    let primary_key = required_expr(
        object_args.primary_key,
        struct_name.span(),
        "ObjectType requires #[tesela(primary_key = ...)]",
    )?;
    let api_name = api_name_tokens(object_args.api_name, &fallback_api_name);
    let datasource = api_name_tokens(Some(datasource), "");
    let primary_key = api_name_tokens(Some(primary_key), "");
    let display = option_lit_tokens(object_args.display);
    let description = option_lit_tokens(object_args.description);

    let fields = named_fields(&input)?;
    let mut property_builders = Vec::with_capacity(fields.len());

    for field in fields {
        let field_name = match &field.ident {
            Some(ident) => ident.to_string(),
            None => {
                return Err(syn::Error::new(
                    field.span(),
                    "ObjectType requires named fields",
                ));
            }
        };
        let ty_str = type_to_string(&field.ty);
        let field_args = parse_field_args(&field.attrs)?;
        let data_type = match &field_args.data_type {
            Some(data_type) => quote!(#data_type),
            None => rust_type_to_data_type(&ty_str),
        };
        let field_nullable = field_args.nullable || is_option(&ty_str);
        let field_description = option_lit_tokens(field_args.description);
        let field_source_column = option_lit_tokens(field_args.source_column);
        let field_indexed = field_args.indexed;
        let field_unique = field_args.unique;
        let field_encrypted = field_args.encrypted;

        property_builders.push(quote! {
            ::tesela::ir::StaticProperty {
                api_name: #field_name,
                display: None,
                description: #field_description,
                data_type: #data_type,
                nullable: #field_nullable,
                indexed: #field_indexed,
                unique: #field_unique,
                source_column: #field_source_column,
                encrypted: #field_encrypted,
            }
        });
    }

    Ok(quote! {
        impl ::tesela::ir::ObjectTypeDefinition for #struct_name {
            fn definition() -> ::tesela::ir::StaticObjectType {
                const PROPERTIES: &[::tesela::ir::StaticProperty] = &[#(#property_builders),*];

                ::tesela::ir::StaticObjectType {
                    api_name: #api_name,
                    display: #display,
                    description: #description,
                    datasource: #datasource,
                    resource: Some(#api_name),
                    primary_key: #primary_key,
                    properties: PROPERTIES,
                    traits: &[],
                    tags: &[],
                    indexes: &[],
                }
            }
        }

        impl #struct_name {
            /// Return the Tesela `ObjectType` definition for this struct.
            pub fn tesela_object_type() -> ::tesela::ir::ObjectType {
                <Self as ::tesela::ir::ObjectTypeDefinition>::object_type()
            }
        }
    })
}

fn named_fields(input: &DeriveInput) -> Result<Vec<&syn::Field>> {
    match &input.data {
        syn::Data::Struct(ds) => match &ds.fields {
            syn::Fields::Named(named) => Ok(named.named.iter().collect()),
            _ => Err(syn::Error::new(
                Span::call_site(),
                "ObjectType requires a struct with named fields",
            )),
        },
        _ => Err(syn::Error::new(
            Span::call_site(),
            "ObjectType can only be applied to structs",
        )),
    }
}

fn parse_object_args(attrs: &[Attribute]) -> Result<ObjectTypeArgs> {
    let mut args = ObjectTypeArgs::default();
    for attr in tesela_attrs(attrs) {
        attr.parse_nested_meta(|meta| {
            if meta.path.is_ident("name") {
                args.api_name = Some(meta.value()?.parse()?);
                return Ok(());
            }
            if meta.path.is_ident("datasource") {
                args.datasource = Some(meta.value()?.parse()?);
                return Ok(());
            }
            if meta.path.is_ident("primary_key") {
                args.primary_key = Some(meta.value()?.parse()?);
                return Ok(());
            }
            if meta.path.is_ident("display") {
                args.display = Some(meta.value()?.parse()?);
                return Ok(());
            }
            if meta.path.is_ident("description") {
                args.description = Some(meta.value()?.parse()?);
                return Ok(());
            }
            Err(meta.error("unsupported ObjectType #[tesela(...)] attribute"))
        })?;
    }
    Ok(args)
}

fn parse_field_args(attrs: &[Attribute]) -> Result<FieldArgs> {
    let mut args = FieldArgs::default();
    for attr in tesela_attrs(attrs) {
        attr.parse_nested_meta(|meta| {
            if meta.path.is_ident("indexed") {
                args.indexed = true;
                return Ok(());
            }
            if meta.path.is_ident("unique") {
                args.unique = true;
                return Ok(());
            }
            if meta.path.is_ident("nullable") {
                args.nullable = true;
                return Ok(());
            }
            if meta.path.is_ident("encrypted") {
                args.encrypted = true;
                return Ok(());
            }
            if meta.path.is_ident("data_type") {
                args.data_type = Some(meta.value()?.parse()?);
                return Ok(());
            }
            if meta.path.is_ident("description") {
                args.description = Some(meta.value()?.parse()?);
                return Ok(());
            }
            if meta.path.is_ident("source_column") {
                args.source_column = Some(meta.value()?.parse()?);
                return Ok(());
            }
            Err(meta.error("unsupported field #[tesela(...)] attribute"))
        })?;
    }
    Ok(args)
}

fn tesela_attrs(attrs: &[Attribute]) -> impl Iterator<Item = &Attribute> {
    attrs.iter().filter(|attr| attr.path().is_ident("tesela"))
}

fn required_expr(value: Option<Expr>, span: Span, message: &'static str) -> Result<Expr> {
    value.ok_or_else(|| syn::Error::new(span, message))
}

fn api_name_tokens(value: Option<Expr>, fallback: &str) -> TokenStream2 {
    match value {
        Some(expr) => quote!(::tesela::core::ApiNameSource::api_name(&(#expr))),
        None => quote!(#fallback),
    }
}

fn option_lit_tokens(value: Option<LitStr>) -> TokenStream2 {
    match value {
        Some(value) => quote!(Some(#value)),
        None => quote!(None),
    }
}
