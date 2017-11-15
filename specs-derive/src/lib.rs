//! Implements the `#[derive(Component)]` macro and `#[component]` attribute for
//! [Specs][sp].
//!
//! [sp]: https://slide-rs.github.io/specs-website/

#![recursion_limit="256"]

extern crate proc_macro;
#[macro_use]
extern crate quote;
extern crate syn;

use proc_macro::TokenStream;
use quote::Tokens;
use syn::{Body, Ident, DeriveInput, MetaItem, NestedMetaItem, VariantData};

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

fn impl_component(ast: &DeriveInput) -> Tokens {
    let name = &ast.ident;
    let (impl_generics, ty_generics, where_clause) = ast.generics.split_for_impl();

    let storage = ast.attrs
        .first()
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
            type Metadata = ();
        }
    }
}

#[proc_macro_derive(Metadata, attributes(metadata))]
pub fn metadata(input: TokenStream) -> TokenStream {
    let s = input.to_string();
    let ast = syn::parse_derive_input(&s).unwrap();
    let gen = impl_metadata(&ast);
    gen.parse().unwrap()
}

fn impl_metadata(ast: &DeriveInput) -> Tokens {
    let name = &ast.ident;
    let (impl_generics, ty_generics, where_clause) = ast.generics.split_for_impl();

    let ref field = match ast.body {
        Body::Enum(_) => panic!("Metadata does not work with enums currently"),
        Body::Struct(ref variant) => match *variant {
            VariantData::Struct(ref fields) => fields.clone(),
            _ => panic!("Unnamed/empty metadata is not supported"),
        }
    };

    let ref field_name = field.iter().filter_map(|field| field.ident.clone()).collect::<Vec<_>>();

    quote! {
        impl<T, #impl_generics> ::specs::Metadata<T> for #name #ty_generics #where_clause {
            fn clean<F>(&mut self, f: &F)
                where
                    F: Fn(Index) -> bool
            {
                #( ::specs::Metadata::<T>::clean(&mut self.#field_name, f); )*
            }
            fn get(&self, id: Index, value: &T) {
                #( ::specs::Metadata::<T>::get(&self.#field_name, id, value); )*
            }
            fn get_mut(&mut self, id: Index, value: &mut T) {
                #( ::specs::Metadata::<T>::get_mut(&mut self.#field_name, id, value); )*
            }
            fn insert(&mut self, id: Index, value: &T) {
                #( ::specs::Metadata::<T>::insert(&mut self.#field_name, id, value); )*
            }
            fn remove(&mut self, id: Index, value: &T) {
                #( ::specs::Metadata::<T>::remove(&mut self.#field_name, id, value); )*
            }
        }
    }
}
