//! Shared helper functions for macro code generation.

use quote::quote;

/// Convert a PascalCase or camelCase identifier to snake_case.
pub(crate) fn to_snake_case(s: &str) -> String {
    let mut out = String::with_capacity(s.len() + 4);
    for (i, ch) in s.chars().enumerate() {
        if ch.is_uppercase() && i > 0 {
            out.push('_');
        }
        out.push(ch.to_ascii_lowercase());
    }
    out
}

/// Map a Rust type path (as a string slice) to a Tesela data type variant.
pub(crate) fn rust_type_to_data_type(ty_str: &str) -> proc_macro2::TokenStream {
    match ty_str
        .trim_start_matches("Option<")
        .trim_end_matches('>')
        .trim()
    {
        "String" | "&str" | "str" => quote!(::tesela::core::DataType::String),
        "i8" | "i16" | "i32" | "i64" | "u8" | "u16" | "u32" | "u64" | "usize" | "isize" => {
            quote!(::tesela::core::DataType::Integer)
        }
        "f32" | "f64" => quote!(::tesela::core::DataType::Float),
        "bool" => quote!(::tesela::core::DataType::Boolean),
        "uuid::Uuid" | "Uuid" => quote!(::tesela::core::DataType::Uuid),
        "serde_json::Value" | "Value" => quote!(::tesela::core::DataType::Json),
        _ => quote!(::tesela::core::DataType::String),
    }
}

/// Return whether the type string represents an `Option<T>`.
pub(crate) fn is_option(ty_str: &str) -> bool {
    ty_str.starts_with("Option<") || ty_str.starts_with("std::option::Option<")
}

/// Stringify a `syn::Type`.
pub(crate) fn type_to_string(ty: &syn::Type) -> String {
    quote!(#ty).to_string().replace(' ', "")
}
