//! `#[action]` attribute macro implementation.

use crate::helpers::type_to_string;
use darling::FromMeta;
use proc_macro::TokenStream;
use quote::{format_ident, quote};
use syn::parse_macro_input;

#[derive(Debug, Default, FromMeta)]
struct ActionArgs {
    #[darling(default)]
    risk: Option<String>,
    #[darling(default)]
    handler: Option<String>,
    #[darling(default)]
    display: Option<String>,
    #[darling(default)]
    description: Option<String>,
}

pub(crate) fn expand(args: TokenStream, input: TokenStream) -> TokenStream {
    let attr_args = match darling::ast::NestedMeta::parse_meta_list(args.into()) {
        Ok(v) => v,
        Err(e) => return TokenStream::from(e.to_compile_error()),
    };
    let macro_args = match ActionArgs::from_list(&attr_args) {
        Ok(v) => v,
        Err(e) => return TokenStream::from(e.write_errors()),
    };

    let input_fn = parse_macro_input!(input as syn::ItemFn);
    let fn_name = &input_fn.sig.ident;
    let fn_name_str = fn_name.to_string();
    let api_name_str = fn_name_str.clone();

    let risk = match macro_args.risk {
        Some(value) => value,
        None => "low".to_string(),
    };
    let handler_kind = match macro_args.handler {
        Some(value) => value,
        None => "callback".to_string(),
    };
    let display_name = match macro_args.display {
        Some(value) => value,
        None => fn_name_str.clone(),
    };
    let mut description = String::new();
    if let Some(value) = macro_args.description {
        description = value;
    }

    let params = &input_fn.sig.inputs;
    let mut schema_props = Vec::new();
    for param in params {
        if let syn::FnArg::Typed(pt) = param
            && let syn::Pat::Ident(pi) = pt.pat.as_ref()
        {
            let param_name = pi.ident.to_string();
            let ty_str = type_to_string(&pt.ty);
            let dt_str = match ty_str.as_str() {
                "String" | "&str" => "string",
                "i8" | "i16" | "i32" | "i64" | "u8" | "u16" | "u32" | "u64" => "integer",
                "f32" | "f64" => "float",
                "bool" => "boolean",
                _ => "string",
            };
            schema_props.push(quote! {
                (#param_name.to_string(), serde_json::json!({"type": #dt_str}))
            });
        }
    }

    let action_struct_name = {
        let mut s = String::new();
        let mut cap_next = true;
        for ch in fn_name_str.chars() {
            if ch == '_' {
                cap_next = true;
            } else if cap_next {
                s.push(ch.to_ascii_uppercase());
                cap_next = false;
            } else {
                s.push(ch);
            }
        }
        s.push_str("Action");
        format_ident!("{}", s)
    };

    let expanded = quote! {
        #input_fn

        /// Generated action type accessor.
        pub struct #action_struct_name;

        impl #action_struct_name {
            /// Return the Tesela `ActionType` definition for this action.
            pub fn tesela_action_type() -> ::tesela_ir::ActionType {
                let props: std::collections::HashMap<String, serde_json::Value> =
                    vec![ #(#schema_props),* ].into_iter().collect();
                ::tesela_ir::ActionType {
                    api_name: ::tesela_core::ApiName::new_unchecked(#api_name_str),
                    display: Some(#display_name.to_string()),
                    description: if #description.is_empty() { None } else { Some(#description.to_string()) },
                    subject: None,
                    handler: ::tesela_ir::ActionHandler {
                        kind: #handler_kind.to_string(),
                        target: Some(#api_name_str.to_string()),
                        config: None,
                    },
                    input_schema: if props.is_empty() {
                        None
                    } else {
                        Some(::tesela_core::Value::new(serde_json::json!({
                            "type": "object",
                            "properties": props,
                        })))
                    },
                    output_schema: None,
                    mode: None,
                    risk_level: Some(#risk.to_string()),
                    idempotency_key: None,
                    deprecated_at: None,
                    metadata: None,
                }
            }
        }
    };

    TokenStream::from(expanded)
}
