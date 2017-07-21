
//! Specs procedural derive macro library.
//!
//! Allows for easy grouping of components and systems into groups for ease of use and modularity.

// `quote` relies on macro recursion, so it is likely to hit the normal cap.
#![recursion_limit = "512"]

extern crate proc_macro;
extern crate syn;
#[macro_use]
extern crate quote;
extern crate specs;
#[cfg(feature="serialize")]
extern crate serde;

use proc_macro::TokenStream;

/// Implementation of the group macros.
mod component_group;

/// Sets up derive for the `ComponentGroup` trait (includes `SerializeGroup`).
#[proc_macro_derive(ComponentGroup, attributes(group))]
pub fn derive_component(input: TokenStream) -> TokenStream {
    let input = syn::parse_derive_input(&input.to_string()).unwrap();
    match component_group::expand_group(&input) {
        Ok(tokens) => tokens.parse().unwrap(),
        Err(err) => panic!("Error: {}", err),
    }
}
