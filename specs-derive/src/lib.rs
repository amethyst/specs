//! Implements the `#[derive(Component)]`, `#[derive(Saveload)]` macro and
//! `#[component]` attribute for [Specs][sp].
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
use syn::{
    parse::{Parse, ParseStream, Result},
    DeriveInput, Path, PathArguments,
};

mod impl_saveload;

/// Custom derive macro for the `Component` trait.
///
/// ## Example
///
/// ```rust,ignore
/// use specs::storage::VecStorage;
///
/// #[derive(Component, Debug)]
/// #[storage(VecStorage<Self>)] // This line is optional, defaults to `DenseVecStorage<Self>`
/// struct Pos(f32, f32, f32);
/// ```
///
/// When the type parameter is `<Self>` it can be omitted i.e.:
///
///```rust,ignore
/// use specs::storage::VecStorage;
///
/// #[derive(Component, Debug)]
/// #[storage(VecStorage)] // Equals to #[storage(VecStorage<Self>)]
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
            storage: content.parse()?,
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
            syn::parse2::<StorageAttribute>(attr.tokens.clone())
                .unwrap()
                .storage
        })
        .unwrap_or_else(|| parse_quote!(::specs::storage::DenseVecStorage));

    let additional_generics = match storage.segments.last().unwrap().arguments {
        PathArguments::AngleBracketed(_) => quote!(),
        _ => quote!(<Self>),
    };

    quote! {
        impl #impl_generics Component for #name #ty_generics #where_clause {
            type Storage = #storage #additional_generics;
        }
    }
}

/// Custom derive macro for the `ConvertSaveload` trait.
///
/// Requires `Entity`, `ConvertSaveload`, `Marker` to be in a scope
///
/// ## Example
///
/// ```rust,ignore
/// use specs::{Entity, saveload::{ConvertSaveload, Marker}};
///
/// #[derive(ConvertSaveload)]
/// struct Target(Entity);
/// ```
#[proc_macro_derive(
    ConvertSaveload,
    attributes(convert_save_load_attr, convert_save_load_skip_convert)
)]
pub fn saveload(input: TokenStream) -> TokenStream {
    use impl_saveload::impl_saveload;
    let mut ast = syn::parse(input).unwrap();

    let gen = impl_saveload(&mut ast);
    gen.into()
}
