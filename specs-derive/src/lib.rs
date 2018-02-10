//! Implements the `#[derive(Component)]` macro and `#[component]` attribute for
//! [Specs][sp].
//!
//! [sp]: https://slide-rs.github.io/specs-website/

extern crate proc_macro;
#[macro_use]
extern crate quote;
#[macro_use]
extern crate syn;

use proc_macro::TokenStream;
use quote::Tokens;
use syn::{Path, DeriveInput};
use syn::synom::Synom;

/// Custom derive macro for the `Component` trait.
///
/// ## Example
///
/// ```rust,ignore
/// use specs::storage::VecStorage;
///
/// #[derive(Component, Debug)]
/// #[storage(VecStorage)]
/// struct Pos(f32, f32, f32);
/// ```
#[proc_macro_derive(Component, attributes(storage))]
pub fn component(input: TokenStream) -> TokenStream {
    let ast = syn::parse(input).unwrap();
    let gen = impl_component(&ast);
    gen.into()
}

struct StorageAttribute {
    storage: Path,
}

impl Synom for StorageAttribute {
    named!(parse -> Self, map!(
        parens!(syn!(Path)),
        |(_, storage)| StorageAttribute { storage }
    ));
}

fn impl_component(ast: &DeriveInput) -> Tokens {
    let name = &ast.ident;
    let (impl_generics, ty_generics, where_clause) = ast.generics.split_for_impl();

    let storage = ast.attrs
        .first()
        .map(|attr| attr.tts.clone())
        //.unwrap_or_else(|| quote!("(DenseVecStorage)").into());
        .unwrap();

    let attr: StorageAttribute = syn::parse2(storage).unwrap();
    let storage = attr.storage;

    quote! {
        impl #impl_generics ::specs::world::Component for #name #ty_generics #where_clause {
            type Storage = #storage<Self>;
        }
    }
}
