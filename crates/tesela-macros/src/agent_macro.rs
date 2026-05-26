//! `#[derive(Agent)]` derive macro implementation.

use crate::helpers::to_snake_case;
use darling::FromAttributes;
use proc_macro::TokenStream;
use quote::quote;
use syn::{DeriveInput, parse_macro_input};

#[derive(Debug, Default, FromAttributes)]
#[darling(attributes(tesela))]
struct AgentArgs {
    #[darling(default)]
    model: Option<String>,
    #[darling(default)]
    display: Option<String>,
}

pub(crate) fn expand(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    let struct_name = &input.ident;
    let struct_name_str = struct_name.to_string();
    let api_name_str = to_snake_case(&struct_name_str);

    let macro_args = AgentArgs::from_attributes(&input.attrs).unwrap_or_default();

    let model = macro_args.model.unwrap_or_default();
    let display_name = macro_args.display.unwrap_or(struct_name_str);

    let model_opt = if model.is_empty() {
        quote!(None)
    } else {
        quote!(Some(#model.to_string()))
    };

    let expanded = quote! {
        impl #struct_name {
            /// Return the Tesela `Agent` definition for this struct.
            pub fn tesela_agent() -> ::tesela_ir::Agent {
                ::tesela_ir::Agent {
                    api_name: ::tesela_core::ApiName::new_unchecked(#api_name_str),
                    display: Some(#display_name.to_string()),
                    description: None,
                    model: #model_opt,
                    model_provider: None,
                    instructions: None,
                    allowed_tools: Vec::new(),
                    custom_tools: Vec::new(),
                    context_sources: Vec::new(),
                    memory: None,
                    limits: None,
                    requires_approval: None,
                    deprecated_at: None,
                    metadata: None,
                    capabilities: Vec::new(),
                    output_schema: None,
                    output_object_type: None,
                }
            }
        }
    };

    TokenStream::from(expanded)
}
