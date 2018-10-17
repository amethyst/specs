//! Implements the `#[derive(Component)]`, `#[derive(Saveload)]` macro and `#[component]` attribute for
//! [Specs][sp].
//!
//! [sp]: https://slide-rs.github.io/specs-website/

#![recursion_limit = "128"]

extern crate proc_macro;
extern crate proc_macro2;
#[macro_use]
extern crate quote;
#[macro_use]
extern crate syn;

use proc_macro::TokenStream;
use syn::{DeriveInput, Path, parse::{Parse, ParseStream, Result}};

mod impl_saveload;

/// Custom derive macro for the `Component` trait.
///
/// ## Example
///
/// ```rust,ignore
/// use specs::storage::VecStorage;
///
/// #[derive(Component, Debug)]
/// #[storage(VecStorage)] // This line is optional, defaults to `DenseVecStorage`
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

impl Parse for StorageAttribute {
    fn parse(input: ParseStream) -> Result<Self> {
        let content;
        let _parenthesized_token = parenthesized!(content in input);

        Ok(StorageAttribute {
            storage: content.parse()?
        })
    }
}

fn impl_component(ast: &DeriveInput) -> proc_macro2::TokenStream {
    let name = &ast.ident;
    let (impl_generics, ty_generics, where_clause) = ast.generics.split_for_impl();

    let storage = ast
        .attrs
        .iter()
        .find(|attr| attr.path.segments[0].ident == "storage")
        .map(|attr| {
            syn::parse2::<StorageAttribute>(attr.tts.clone())
                .unwrap()
                .storage
        })
        .unwrap_or_else(|| parse_quote!(DenseVecStorage));

    quote! {
        impl #impl_generics Component for #name #ty_generics #where_clause {
            type Storage = #storage<Self>;
        }
    }
}

/// Custom derive macro for the `ConvertSaveload` trait.
/// 
/// Requires `Entity`, `ConvertSaveload`, `Marker` and `NoError` to be in a scope.
///
/// ## Example
///
/// ```rust,ignore
/// use specs::{Entity, saveload::{ConvertSaveload, Marker}, error::NoError};
/// 
/// #[derive(ConvertSaveload)]
/// struct Target(Entity);
/// ```
#[proc_macro_derive(ConvertSaveload)]
pub fn saveload(input: TokenStream) -> TokenStream {
    use impl_saveload::impl_saveload;
    let mut ast = syn::parse(input).unwrap();

    let gen = impl_saveload(&mut ast);
    gen.into()
}
