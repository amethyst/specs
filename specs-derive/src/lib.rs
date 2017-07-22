//! Implements the `#[derive(Component)]` macro and `#[component]` attribute for
//! [Specs][sp].
//!
//! [sp]: https://slide-rs.github.io/specs-website/

extern crate proc_macro;
#[macro_use]
extern crate quote;
extern crate syn;

use proc_macro::TokenStream;
use syn::{Ident, MacroInput, MetaItem, NestedMetaItem};
use quote::Tokens;

/// Custom derive macro for the `Component` trait.
///
/// ## Example
///
/// ```rust,ignore
/// #[derive(Component, Debug)]
/// struct Pos(f32, f32, f32);
/// ```
///
/// The macro will store components in `DenseVecStorage`s by default. To specify
/// a different storage type, you may use the `#[component]` attribute.
///
/// ```rust,ignore
/// #[derive(Component, Debug)]
/// #[component(HashMapStorage)]
/// struct Pos(f32, f32, f32);
/// ```
#[proc_macro_derive(Component, attributes(component))]
pub fn component(input: TokenStream) -> TokenStream {
    let s = input.to_string();
    let ast = syn::parse_derive_input(&s).unwrap();
    let gen = impl_component(&ast);
    gen.parse().unwrap()
}

fn impl_component(ast: &MacroInput) -> Tokens {
    let name = &ast.ident;
    let (impl_generics, ty_generics, where_clause) = ast.generics.split_for_impl();

    let storage = ast.attrs.first()
        .and_then(|attr| match attr.value {
            MetaItem::List(ref ident, ref items) if ident == "component" => items.first(),
            _ => None,
        })
        .and_then(|attr| match *attr {
            NestedMetaItem::MetaItem(ref item) => Some(item),
            _ => None,
        })
        .and_then(|attr| match *attr {
            MetaItem::Word(ref ident) => Some(ident),
            _ => None,
        })
        .cloned()
        .unwrap_or(Ident::new("::specs::DenseVecStorage"));

    quote! {
        impl #impl_generics ::specs::Component for #name #ty_generics #where_clause {
            type Storage = #storage<#name>;
        }
    }
}
