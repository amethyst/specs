
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

mod component_group;

#[proc_macro_derive(ComponentGroup, attributes(group))]
pub fn derive_component(input: TokenStream) -> TokenStream {
    let input = syn::parse_derive_input(&input.to_string()).unwrap();

    println!("Output: {:?}", component_group::expand_group(&input));

    match component_group::expand_group(&input) {
        Ok(tokens) => tokens.parse().unwrap(),
        Err(err) => panic!("Error: {}", err),
    }
}
