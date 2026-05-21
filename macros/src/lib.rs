use proc_macro::TokenStream;
use quote::quote;
use syn::{ItemFn, parse_macro_input};

/// Marks an async function as spawnable on a Web Worker.
///
/// Generates a `#[wasm_bindgen]`-exported `__register_<fn_name>` function
/// that registers the erased wrapper into worxide's registry.
/// `worxide::init()` scans wasm exports for the `__register_` prefix and
/// calls each one automatically — no manual registration needed.
///
/// # Requirements
/// - The function must be `async`
/// - Input and output types must implement `serde::Serialize + serde::DeserializeOwned`
///
/// # Example
/// ```rust
/// #[worxide::worker_fn]
/// async fn add_ten(n: u32) -> u32 {
///     n + 10
/// }
/// ```
#[proc_macro_attribute]
pub fn worker_fn(_attr: TokenStream, item: TokenStream) -> TokenStream {
    let input = parse_macro_input!(item as ItemFn);

    let fn_name = &input.sig.ident;
    let fn_name_str = fn_name.to_string();

    // Derive the register fn name: __register_add_ten
    let register_name = syn::Ident::new(
        &format!("__register_{}", fn_name),
        fn_name.span(),
    );

    // Extract input type — we require exactly one argument.
    let input_type = match input.sig.inputs.first() {
        Some(syn::FnArg::Typed(pat)) => &*pat.ty,
        _ => panic!("#[worker_fn] requires exactly one argument"),
    };

    // Extract output type from `async fn foo(...) -> T`.
    let output_type = match &input.sig.output {
        syn::ReturnType::Type(_, ty) => quote! { #ty },
        syn::ReturnType::Default => quote! { () },
    };

    let input_type = quote! { #input_type };

    let expanded = quote! {
        // Keep the original function untouched.
        #input

        // Exported registration shim — called by worxide::init() on both
        // main thread and worker via JS reflection on wasm exports.
        #[wasm_bindgen::prelude::wasm_bindgen]
        pub fn #register_name() {
            worxide::register(#fn_name_str, |bytes| {
                ::std::boxed::Box::pin(async move {
                    let input: #input_type = serde_json::from_slice(&bytes).unwrap();
                    let output: #output_type = #fn_name(input).await;
                    serde_json::to_vec(&output).unwrap()
                })
            });
        }
    };

    TokenStream::from(expanded)
}
