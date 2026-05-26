//! `#[policy]` attribute macro implementation.

use darling::FromMeta;
use proc_macro::TokenStream;
use quote::{format_ident, quote};
use syn::parse_macro_input;

#[derive(Debug, Default, FromMeta)]
struct PolicyArgs {
    #[darling(default)]
    effect: Option<String>,
    #[darling(default)]
    roles: Option<String>,
    #[darling(default)]
    operations: Option<String>,
    #[darling(default)]
    resource_kind: Option<String>,
    #[darling(default)]
    resource: Option<String>,
    #[darling(default)]
    description: Option<String>,
    #[darling(default)]
    priority: Option<i32>,
}

pub(crate) fn expand(args: TokenStream, input: TokenStream) -> TokenStream {
    let attr_args = match darling::ast::NestedMeta::parse_meta_list(args.into()) {
        Ok(v) => v,
        Err(e) => return TokenStream::from(e.to_compile_error()),
    };
    let macro_args = match PolicyArgs::from_list(&attr_args) {
        Ok(v) => v,
        Err(e) => return TokenStream::from(e.write_errors()),
    };

    let input_fn = parse_macro_input!(input as syn::ItemFn);
    let fn_name = &input_fn.sig.ident;
    let fn_name_str = fn_name.to_string();

    let effect_str = macro_args.effect.as_deref().unwrap_or("allow");
    let effect_tokens = match effect_str {
        "deny" => quote!(::tesela_core::PolicyEffect::Deny),
        _ => quote!(::tesela_core::PolicyEffect::Allow),
    };

    let roles_tokens = if let Some(ref roles_str) = macro_args.roles {
        let items: Vec<_> = roles_str
            .split(',')
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty())
            .collect();
        quote!(vec![ #(#items.to_string()),* ])
    } else {
        quote!(Vec::new())
    };

    let ops_tokens = if let Some(ref ops_str) = macro_args.operations {
        let items: Vec<_> = ops_str
            .split(',')
            .map(|s| {
                let s = s.trim();
                match s {
                    "search" => quote!(::tesela_core::Operation::Search),
                    "read" => quote!(::tesela_core::Operation::Read),
                    "mutate" => quote!(::tesela_core::Operation::Mutate),
                    "traverse" => quote!(::tesela_core::Operation::Traverse),
                    "aggregate" => quote!(::tesela_core::Operation::Aggregate),
                    "upload" => quote!(::tesela_core::Operation::Upload),
                    "execute" => quote!(::tesela_core::Operation::Execute),
                    _ => quote!(::tesela_core::Operation::Read),
                }
            })
            .collect();
        quote!(vec![ #(#items),* ])
    } else {
        quote!(Vec::new())
    };

    let resource_kind_tokens = match &macro_args.resource_kind {
        Some(rk) => quote!(Some(#rk.to_string())),
        None => quote!(None),
    };

    let resource_tokens = match &macro_args.resource {
        Some(r) => quote!(Some(::tesela_core::ApiName::new_unchecked(#r))),
        None => quote!(None),
    };

    let description_tokens = match &macro_args.description {
        Some(d) if !d.is_empty() => quote!(Some(#d.to_string())),
        _ => quote!(None),
    };

    let priority_tokens = match macro_args.priority {
        Some(p) => quote!(Some(#p)),
        None => quote!(None),
    };

    let policy_struct_name = {
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
        s.push_str("Policy");
        format_ident!("{}", s)
    };

    let expanded = quote! {
        #input_fn

        /// Generated policy rule accessor.
        pub struct #policy_struct_name;

        impl #policy_struct_name {
            /// Return the Tesela `PolicyRule` definition for this policy.
            pub fn tesela_policy_rule() -> ::tesela_ir::PolicyRule {
                ::tesela_ir::PolicyRule {
                    api_name: ::tesela_core::ApiName::new_unchecked(#fn_name_str),
                    description: #description_tokens,
                    effect: #effect_tokens,
                    actors: Vec::new(),
                    roles: #roles_tokens,
                    operations: #ops_tokens,
                    resource_kind: #resource_kind_tokens,
                    resource: #resource_tokens,
                    condition: None,
                    row_filter: None,
                    redactions: Vec::new(),
                    obligations: Vec::new(),
                    priority: #priority_tokens,
                }
            }
        }
    };

    TokenStream::from(expanded)
}
