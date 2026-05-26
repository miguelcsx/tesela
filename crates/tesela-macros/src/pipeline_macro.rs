//! `#[derive(Pipeline)]` derive macro implementation.

use crate::helpers::to_snake_case;
use darling::FromAttributes;
use proc_macro::TokenStream;
use quote::quote;
use syn::{DeriveInput, parse_macro_input};

#[derive(Debug, Default, FromAttributes)]
#[darling(attributes(tesela))]
struct PipelineArgs {
    #[darling(default)]
    schedule: Option<String>,
    #[darling(default)]
    mode: Option<String>,
    #[darling(default)]
    display: Option<String>,
}

pub(crate) fn expand(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    let struct_name = &input.ident;
    let struct_name_str = struct_name.to_string();
    let api_name_str = to_snake_case(&struct_name_str);

    let macro_args = PipelineArgs::from_attributes(&input.attrs).unwrap_or_default();
    let display_name = macro_args.display.unwrap_or(struct_name_str);

    let mode_tokens = match macro_args.mode.as_deref() {
        Some("snapshot") => quote!(::tesela_ir::ExecutionMode::Snapshot),
        _ => quote!(::tesela_ir::ExecutionMode::Incremental),
    };

    let schedule_tokens = match &macro_args.schedule {
        Some(s) if s == "manual" => quote!(Some(::tesela_ir::PipelineSchedule::Manual)),
        Some(s) => quote!(Some(::tesela_ir::PipelineSchedule::Cron(#s.to_string()))),
        None => quote!(None),
    };

    let expanded = quote! {
        impl #struct_name {
            /// Return the Tesela `TransformPipeline` definition for this struct.
            ///
            /// The struct must implement a `fn steps() -> Vec<TransformStep>` method
            /// providing the pipeline steps.
            pub fn tesela_pipeline() -> ::tesela_ir::TransformPipeline {
                ::tesela_ir::TransformPipeline {
                    api_name: ::tesela_core::ApiName::new_unchecked(#api_name_str),
                    display: Some(#display_name.to_string()),
                    description: None,
                    steps: Self::steps(),
                    schedule: #schedule_tokens,
                    mode: #mode_tokens,
                    context: None,
                    metadata: None,
                }
            }
        }
    };

    TokenStream::from(expanded)
}
