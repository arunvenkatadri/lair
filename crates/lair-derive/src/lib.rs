//! LAIR derive macros.
//!
//! - `#[lair_task]` — auto-implements `Freezable` (stateless default) on a struct.
//! - `#[lair_runtime(config = "...")]` — thin wrapper that emits `#[cu29_derive::copper_runtime(...)]`.

use proc_macro::TokenStream;
use quote::quote;
use syn::{parse_macro_input, ItemStruct};

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
/// Delegates to `#[cu29_derive::copper_runtime(...)]`.
///
/// # Example
///
/// ```ignore
/// #[lair_runtime(config = "robot.ron")]
/// struct App {}
/// ```
#[proc_macro_attribute]
pub fn lair_runtime(attr: TokenStream, item: TokenStream) -> TokenStream {
    // Re-emit with the copper_runtime attribute — Rust expands iteratively.
    let attr2: proc_macro2::TokenStream = attr.into();
    let item2: proc_macro2::TokenStream = item.into();

    let expanded = quote! {
        #[cu29_derive::copper_runtime(#attr2)]
        #item2
    };

    expanded.into()
}
