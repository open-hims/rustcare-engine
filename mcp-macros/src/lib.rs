//! MCP Tool decorator macros (proc macro crate)
//!
//! Provides attribute macros similar to utoipa for automatically generating
//! MCP tools from handler functions with auth and Zanzibar context.

use proc_macro::TokenStream;
use quote::quote;
use syn::{parse_macro_input, ItemFn, Meta, Lit, NestedMeta};

/// Decorator macro to mark a handler function as an MCP tool
///
/// Usage:
/// ```rust
/// #[mcp_tool(
///     name = "get_patient",
///     description = "Retrieve patient information by ID",
///     category = "healthcare",
///     requires_permission = "patient:read",
///     sensitive = false,
///     response_type = "Patient",
///     render_type = "json"
/// )]
/// pub async fn get_patient(...) -> Result<...> {
///     // handler implementation
/// }
/// ```
#[proc_macro_attribute]
pub fn mcp_tool(args: TokenStream, input: TokenStream) -> TokenStream {
    let input_fn = parse_macro_input!(input as ItemFn);
    let attr_args = parse_macro_input!(args as syn::AttributeArgs);
    
    // Parse the mcp_tool attributes
    let mut tool_name = None;
    let mut description = None;
    let mut category = None;
    let mut requires_permission = None;
    let mut sensitive = None;
    let mut response_type = None;
    let mut render_type = None;
    
    for arg in attr_args {
        if let NestedMeta::Meta(Meta::NameValue(meta)) = arg {
            let ident = meta.path.get_ident().map(|i| i.to_string());
            
            if let Some(ident_str) = ident {
                match ident_str.as_str() {
                    "name" => {
                        if let syn::Expr::Lit(syn::ExprLit { lit: Lit::Str(s), .. }) = meta.value {
                            tool_name = Some(s.value());
                        }
                    }
                    "description" => {
                        if let syn::Expr::Lit(syn::ExprLit { lit: Lit::Str(s), .. }) = meta.value {
                            description = Some(s.value());
                        }
                    }
                    "category" => {
                        if let syn::Expr::Lit(syn::ExprLit { lit: Lit::Str(s), .. }) = meta.value {
                            category = Some(s.value());
                        }
                    }
                    "requires_permission" => {
                        if let syn::Expr::Lit(syn::ExprLit { lit: Lit::Str(s), .. }) = meta.value {
                            requires_permission = Some(s.value());
                        }
                    }
                    "sensitive" => {
                        if let syn::Expr::Lit(syn::ExprLit { lit: Lit::Bool(b), .. }) = meta.value {
                            sensitive = Some(b.value);
                        }
                    }
                    "response_type" => {
                        if let syn::Expr::Lit(syn::ExprLit { lit: Lit::Str(s), .. }) = meta.value {
                            response_type = Some(s.value());
                        }
                    }
                    "render_type" => {
                        if let syn::Expr::Lit(syn::ExprLit { lit: Lit::Str(s), .. }) = meta.value {
                            render_type = Some(s.value());
                        }
                    }
                    _ => {}
                }
            }
        }
    }
    
    // Default values
    let tool_name = tool_name.unwrap_or_else(|| {
        // Default to function name with underscores
        input_fn.sig.ident.to_string()
    });
    
    let description = description.unwrap_or_else(|| format!("Execute {}", tool_name));
    let category = category.unwrap_or_else(|| "general".to_string());
    let requires_permission_str = requires_permission.as_ref().map(|s| quote! { Some(#s) }).unwrap_or_else(|| quote! { None });
    let sensitive_bool = sensitive.unwrap_or(false);
    let response_type_str = response_type.as_ref().map(|s| quote! { Some(#s) }).unwrap_or_else(|| quote! { None });
    
    // Parse render_type string to RenderType enum
    let render_type_enum = render_type.as_ref().map(|rt_str| {
        match rt_str.as_str() {
            "json" => quote! { Some(::mcp_server::protocol::RenderType::Json) },
            "markdown" => quote! { Some(::mcp_server::protocol::RenderType::Markdown) },
            "html" => quote! { Some(::mcp_server::protocol::RenderType::Html) },
            "table" => quote! { Some(::mcp_server::protocol::RenderType::Table) },
            "list" => quote! { Some(::mcp_server::protocol::RenderType::List) },
            "text" => quote! { Some(::mcp_server::protocol::RenderType::Text) },
            _ => quote! { Some(::mcp_server::protocol::RenderType::Json) }, // Default to JSON
        }
    }).unwrap_or_else(|| quote! { None });
    
    // Generate the tool registration code
    let fn_name = &input_fn.sig.ident;
    let fn_vis = &input_fn.vis;
    let fn_attrs = &input_fn.attrs;
    let fn_sig = &input_fn.sig;
    let fn_block = &input_fn.block;
    
    let expanded = quote! {
        #(#fn_attrs)*
        #fn_vis #fn_sig {
            #fn_block
        }
        
        // Auto-generated MCP tool metadata
        // This function is registered as an MCP tool with the following metadata:
        // - Name: #tool_name
        // - Description: #description
        // - Category: #category
        // - Required Permission: #requires_permission_str
        // - Sensitive: #sensitive_bool
        // - Response Type: #response_type_str
        // - Render Type: #render_type_enum
        //
        // The tool will be automatically discovered by build.rs scanning for #[mcp_tool] attributes
    };
    
    TokenStream::from(expanded)
}

