//! LAIR derive macros.
//!
//! - `#[lair_task]` — auto-implements `Freezable` (stateless default) on a struct.
//! - `#[lair_runtime(config = "...")]` — thin wrapper that emits `#[cu29_derive::copper_runtime(...)]`.

use proc_macro::TokenStream;
use quote::quote;
use syn::parse::Parser;
use syn::punctuated::Punctuated;
use syn::{parse_macro_input, ItemStruct, Lit, Meta, Token};

/// Extracts the `config = "..."` path from the `lair_runtime` attribute args,
/// best-effort (returns `None` if the args don't parse or `config` is absent).
fn extract_config_path(attr: &proc_macro2::TokenStream) -> Option<String> {
    let metas = Punctuated::<Meta, Token![,]>::parse_terminated
        .parse2(attr.clone())
        .ok()?;
    for meta in metas {
        if let Meta::NameValue(nv) = meta {
            if nv.path.is_ident("config") {
                if let syn::Expr::Lit(expr) = nv.value {
                    if let Lit::Str(s) = expr.lit {
                        return Some(s.value());
                    }
                }
            }
        }
    }
    None
}

/// Applied to a task struct to auto-implement `Freezable` with stateless defaults.
///
/// # Example
///
/// ```ignore
/// #[lair_task]
/// pub struct MySensor {
///     count: u32,
/// }
/// // Expands to:
/// // impl Freezable for MySensor {}
/// ```
#[proc_macro_attribute]
pub fn lair_task(_attr: TokenStream, item: TokenStream) -> TokenStream {
    let input = parse_macro_input!(item as ItemStruct);
    let name = &input.ident;
    let (impl_generics, ty_generics, where_clause) = input.generics.split_for_impl();

    let expanded = quote! {
        #input

        impl #impl_generics Freezable for #name #ty_generics #where_clause {}
    };

    expanded.into()
}

/// Applied to a struct to generate the Copper runtime wiring.
///
/// Delegates to the `copper_runtime` attribute macro, which must be in scope —
/// it is re-exported from the LAIR prelude, so `use cu29::prelude::*;` (or
/// `use lair::prelude::*;`) is enough. This keeps consumers from having to depend
/// on the internal `cu29-derive` crate directly.
///
/// # Example
///
/// ```ignore
/// use cu29::prelude::*;
///
/// #[lair_runtime(config = "robot.ron")]
/// struct App {}
/// ```
#[proc_macro_attribute]
pub fn lair_runtime(attr: TokenStream, item: TokenStream) -> TokenStream {
    // Re-emit with the copper_runtime attribute — Rust expands iteratively.
    // Emitting the bare (in-scope) name avoids forcing a direct `cu29-derive`
    // dependency on every LAIR application crate.
    let attr2: proc_macro2::TokenStream = attr.into();
    let item2: proc_macro2::TokenStream = item.into();

    // Compile-time safety check: refuse to build a graph in which a ControlCommand
    // reaches an actuator without a SafetyGuardTask in front of it. Best-effort —
    // if the config can't be located or parsed here, we defer to Copper's own
    // codegen and skip the check rather than failing the build spuriously.
    if let Some(config) = extract_config_path(&attr2) {
        if let Ok(manifest_dir) = std::env::var("CARGO_MANIFEST_DIR") {
            let full_path = std::path::Path::new(&manifest_dir).join(&config);
            if let Some(path_str) = full_path.to_str() {
                if let Err(msg) = lair_biscuit::audit_file(path_str) {
                    return syn::Error::new(proc_macro2::Span::call_site(), msg)
                        .to_compile_error()
                        .into();
                }
            }
        }
    }

    let expanded = quote! {
        #[copper_runtime(#attr2)]
        #item2
    };

    expanded.into()
}
