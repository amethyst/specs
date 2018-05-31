//! Contains implementations for #[derive(Saveload)]
//! which derives both `IntoSerialize` and `FromDeserialize`.
//!
//! Since this requires a proxy "Data" `struct`/`enum`, these two need
//! to be derived together or it begins requiring unwieldy attribute explosion.
//!
//! Currently there is one major limitation to this implementation: it cannot handle
//! generic types such as `struct Foo<T> { x: T, e: Entity }`. I'm not good enough at
//! using `syn` to iron this out, but since specs requires explicit references to non-generic
//! components in system data, it can generally be worked around by newtyping like
//! `struct TypedFoo(Foo<u32>);` and the like.

// NOTE: All examples given in the documentation below are "cleaned up" into readable Rust,
// so it doesn't give an entirely accurate view of what's actually generated (for instance,
// I'll write `Entity` instead of `::specs::Entity` which is actually what's generated).

use quote::Tokens;
use syn::{DataEnum, DataStruct, DeriveInput, Field, Generics, Ident, Type, TypeParamBound};

// Handy collection since tuples got unwieldy and
// unclear in purpose
struct SaveloadDerive {
    type_def: Tokens,
    ser: Tokens,
    de: Tokens,
    saveload_name: Ident,
    saveload_generics: Generics,
}

/// The main entrypoint, sets things up and delegates to the
/// type we're deriving on.
pub fn impl_saveload(ast: &mut DeriveInput) -> Tokens {
    use syn::Data;

    if !ast.generics.params.is_empty() {
        panic!("Deriving `Saveload` does not, at the moment, support generic types (i.e. `struct Foo<T>`).
        
        This limitation may be fixed in the future, but for now you can usually work around this by newtyping
        your generic data structure to a concrete-variant (e.g. `struct FooU32(Foo<u32>);`), or implementing `IntoSerialize` and `FromDeserialize` 
        yourself, which is relatively straightforward if tedious (see the documentation for those traits).")
    }

    add_bound(
        &mut ast.generics,
        parse_quote!(::specs::saveload::IntoSerialize),
    );
    add_bound(
        &mut ast.generics,
        parse_quote!(::specs::saveload::FromDeserialize),
    );
    add_bound(&mut ast.generics, parse_quote!(Clone));

    let derive = match ast.data {
        Data::Struct(ref mut data) => saveload_struct(data, &mut ast.ident, &mut ast.generics),
        Data::Enum(ref data) => saveload_enum(data, &ast.ident, &ast.generics),
        Data::Union(_) => panic!("Unions cannot derive IntoSerialize/FromDeserialize at present"),
    };

    let name = ast.ident;

    let mut impl_generics = ast.generics.clone();
    impl_generics.params.push(parse_quote!(
        MA: ::serde::Serialize + ::serde::de::DeserializeOwned + ::specs::saveload::Marker
    ));
    let (impl_generics, _, where_clause) = impl_generics.split_for_impl();
    let (_, ty_generics, _) = ast.generics.split_for_impl(); // We don't want the type generics we just made
                                                             // because they have MA which our normal type doesn't have

    let type_def = derive.type_def;
    let saveload_name = derive.saveload_name;
    let saveload_generics = derive.saveload_generics;

    let ser = derive.ser;
    let de = derive.de;

    let tt = quote!{
        #[derive(Serialize, Deserialize, Clone)]
        #[serde(bound = "MA: ::specs::saveload::Marker")]
        pub #type_def

        impl #impl_generics ::specs::saveload::IntoSerialize<MA> for #name #ty_generics #where_clause {
            type Data = #saveload_name #saveload_generics;
            type Error = ::specs::error::NoError;

            fn into<F>(&self, mut ids: F) -> Result<Self::Data, Self::Error>
            where
                F: FnMut(::specs::Entity) -> Option<MA>
            {
                #ser
            }
        }

        impl #impl_generics ::specs::saveload::FromDeserialize<MA> for #name #ty_generics #where_clause {
            type Data = #saveload_name #saveload_generics;
            type Error = ::specs::error::NoError;

            fn from<F>(data: Self::Data, mut ids: F) -> Result<Self, Self::Error>
            where
                F: FnMut(MA) -> Option<Entity>
            {
                #de
            }
        }
    };

    // panic!("{}", tt);

    tt
}

// Implements all elements of saveload common to structs of any type
// (except unit structs like `struct Foo;` since they're auto-saveload);
fn saveload_struct(
    data: &mut DataStruct,
    name: &mut Ident,
    generics: &mut Generics,
) -> SaveloadDerive {
    use syn::Fields;

    let mut saveload_generics = generics.clone();
    saveload_generics.params.push(parse_quote!(MA));
    match saveload_generics.where_clause {
        Some(ref mut clause) => clause.predicates.push(parse_quote!(
            MA: ::serde::Serialize + ::serde::de::DeserializeOwned + ::specs::saveload::Marker,
            for <'deser> MA: ::serde::Deserialize<'deser>
        )),
        ref mut clause @ None => {
            *clause = Some(parse_quote!(
            where MA: ::serde::Serialize + ::serde::de::DeserializeOwned + ::specs::saveload::Marker,
            for <'deser> MA: ::serde::Deserialize<'deser>
        ))
        }
    }

    let saveload_name = Ident::new(&format!("{}SaveloadData", name), name.span);

    let saveload_fields: Vec<_> = data
        .fields
        .iter()
        .cloned()
        .map(|mut f| {
            replace_entity_type(&mut f.ty);
            f
        })
        .collect();

    let (struct_def, ser, de) = if let Fields::Named(_) = data.fields {
        saveload_named_struct(&name, &saveload_name, &saveload_generics, &saveload_fields)
    } else if let Fields::Unnamed(_) = data.fields {
        saveload_tuple_struct(
            &data,
            &name,
            &saveload_name,
            &saveload_generics,
            &saveload_fields,
        )
    } else {
        panic!("This derive does not support unit structs (Hint: they automatically derive FromDeserialize and IntoSerialize)");
    };

    SaveloadDerive {
        type_def: struct_def,
        ser,
        de,
        saveload_name,
        saveload_generics,
    }
}

// Automatically derives the two traits and proxy `Data` container for a struct with named fields (e.g. `struct Foo {e: Entity}`).
//
// This generates a struct named `StructNameSaveloadData` such that all fields are their associated `Data` variants, as well as a bound on the required marker
//  e.g.
//
// ```
// struct FooSaveloadData<MA> where MA: Serialize+Marker, for<'de> MA: Deserialize<'de> {
//    e: <Entity as IntoSerialize<MA>>::Data
// }
// ```
//
// The generation for the `into` and `from` functions just constructs each field by calling `into`/`from` for each field in the input struct
// and placing it into the output struct:
//
// ```
//  fn into<F: FnMut(Entity) -> Option<MA>>(&self, mut ids: F) -> Result<Self::Data, Self::Error> {
//      FooSaveloadData {
//          e: IntoSerialize::into(&self.e, &mut ids)?
//      }
//  }
// ```
//
// And similar for `FromDeserialize`.
fn saveload_named_struct(
    name: &Ident,
    saveload_name: &Ident,
    generics: &Generics,
    saveload_fields: &[Field],
) -> (Tokens, Tokens, Tokens) {
    let (_, ty_generics, where_clause) = generics.split_for_impl();

    let struct_def = quote!{
        struct #saveload_name #ty_generics #where_clause {
            #( #saveload_fields ),*
        }
    };

    let field_names = saveload_fields.iter().map(|f| f.ident.clone());
    let field_names_2 = field_names.clone();
    let ser = quote!{
        Ok(#saveload_name {
                # ( #field_names: ::specs::saveload::IntoSerialize::into(&self.#field_names_2, &mut ids)? ),*
            })
    };

    let field_names = saveload_fields.iter().map(|f| f.ident.clone());
    let field_names_2 = field_names.clone();
    let de = quote! { Ok(#name {
                # ( #field_names: ::specs::saveload::FromDeserialize::from(data.#field_names_2, &mut ids)? ),*
            })
    };

    (struct_def, ser, de)
}

// Automatically derives the two traits and proxy `Data` container for a struct with unnamed fields aka a tuple struct (e.g. `struct Foo(Entity);`).
//
// This generates a struct named `StructNameSaveloadData` such that all fields are their associated `Data` variants, as well as a bound on the required marker
//  e.g.
//
// ```
// struct FooSaveloadData<MA>  (
//    <Entity as IntoSerialize<MA>>::Data
// ) where MA: Serialize+Marker, for<'de> MA: Deserialize<'de>;
// ```
//
// The generation for the `into` and `from` functions just constructs each field by calling `into`/`from` for each field in the input struct
// and placing it into the output struct:
//
// ```
//  fn into<F: FnMut(Entity) -> Option<MA>>(&self, mut ids: F) -> Result<Self::Data, Self::Error> {
//      FooSaveloadData (
//          e: IntoSerialize::into(&self.0, &mut ids)?
//      )
//  }
// ```
//
// And similar for `FromDeserialize`.
fn saveload_tuple_struct(
    data: &DataStruct,
    name: &Ident,
    saveload_name: &Ident,
    generics: &Generics,
    saveload_fields: &[Field],
) -> (Tokens, Tokens, Tokens) {
    use syn::Index;

    let (_, ty_generics, where_clause) = generics.split_for_impl();

    let struct_def = quote!{
        struct #saveload_name #ty_generics (
            #( #saveload_fields ),*
        ) #where_clause;
    };

    let field_ids = saveload_fields.iter().enumerate().map(|(i, _)| Index {
        index: i as u32,
        span: data.struct_token.0.clone(),
    });
    let ser = quote!{
        Ok(#saveload_name (
                # ( ::specs::saveload::IntoSerialize::into(&self.#field_ids, &mut ids)? ),*
            ))
    };

    let field_ids = saveload_fields.iter().enumerate().map(|(i, _)| Index {
        index: i as u32,
        span: data.struct_token.0.clone(),
    });
    let de = quote! { Ok(#name (
                # ( ::specs::saveload::FromDeserialize::from(data.#field_ids, &mut ids)? ),*
            ))
    };

    (struct_def, ser, de)
}

// Automatically derives the two traits and proxy `Data` container for an Enum (e.g. `enum Foo{ Bar(Entity), Baz{ e: Entity }, Unit }`).
//
// This will properly handle enum variants with no `Entity`, so long as at least one variant (or one of that variant's fields recursively) contain an
// `Entity` somewhere. If this isn't true, `Saveload` is auto-derived so long as the fields can be `Serialize` and `Deserialize` anyway.
//
// This generates a struct named `EnumNameSaveloadData` such that all fields are their associated `Data` variants, as well as a bound on the required marker
//  e.g.
//
// ```
// enum FooSaveloadData<MA> where MA: Serialize+Marker, for<'de> MA: Deserialize<'de> {
//    Bar(<Entity as IntoSerialize<MA>>::Data),
//    Baz { e: <Entity as IntoSerialize<MA>>::Data },
//    Unit
// };
// ```
//
// The generation for the `into` and `from` functions just constructs each field of each variant by calling `into`/`from` for each field in the input
// and placing it into the output struct in a giant match of each possibility:
//
// ```
//  fn into<F: FnMut(Entity) -> Option<MA>>(&self, mut ids: F) -> Result<Self::Data, Self::Error> {
//      match *self {
//          Foo::Bar(ref field0) => FooSaveloadData::Bar(IntoSerialize::into(field0, &mut ids)? ),
//          Foo::Baz{ ref e } => FooSaveloadData::Baz{ e: IntoSerialize::into(e, &mut ids)? },
//          Foo::Unit => FooSaveloadData::Unit,
//      }
//  }
// ```
//
// And similar for `FromDeserialize`.
fn saveload_enum(data: &DataEnum, name: &Ident, generics: &Generics) -> SaveloadDerive {
    use syn::Fields;

    let mut saveload_generics = generics.clone();
    saveload_generics.params.push(parse_quote!(MA));

    match saveload_generics.where_clause {
        Some(ref mut clause) => clause.predicates.push(parse_quote!(
            MA: ::serde::Serialize + ::serde::de::DeserializeOwned + ::specs::saveload::Marker,
            for <'deser> MA: ::serde::Deserialize<'deser>
        )),
        ref mut clause @ None => {
            *clause = Some(parse_quote!(
            where MA: ::serde::Serialize + ::serde::de::DeserializeOwned + ::specs::saveload::Marker,
            for <'deser> MA: ::serde::Deserialize<'deser>
        ))
        }
    }

    let saveload_name = Ident::new(&format!("{}SaveloadData", name), name.span);

    let mut saveload = data.clone();

    for variant in saveload.variants.iter_mut() {
        if let Fields::Unnamed(ref mut fields) = variant.fields {
            for f in fields.unnamed.iter_mut() {
                replace_entity_type(&mut f.ty);
            }
        } else if let Fields::Named(ref mut fields) = variant.fields {
            for f in fields.named.iter_mut() {
                replace_entity_type(&mut f.ty);
            }
        }
    }

    let variants = &saveload.variants;

    let (_, saveload_ty_generics, saveload_where_clause) = saveload_generics.split_for_impl();
    let enum_def = quote!{
        enum #saveload_name #saveload_ty_generics #saveload_where_clause {
            #( #variants ),*
        }
    };

    let mut big_match_ser = quote!{};
    let mut big_match_de = quote!{};

    for variant in variants {
        let ident = &variant.ident;

        match variant.fields {
            Fields::Named(ref fields) => {
                let names = fields.named.iter().map(|f| f.ident);
                let names_2 = fields.named.iter().map(|f| f.ident);
                let names_3 = fields.named.iter().map(|f| f.ident);

                big_match_ser = quote!{
                    #big_match_ser
                    #name::#ident { #( ref #names ),* } => #saveload_name::#ident { #( #names_3: ::specs::saveload::IntoSerialize::into(#names_2, ids)? ),* },
                };

                let names = fields.named.iter().map(|f| f.ident);
                let names_2 = fields.named.iter().map(|f| f.ident);
                let names_3 = fields.named.iter().map(|f| f.ident);

                big_match_de = quote!{
                    #big_match_de
                    #saveload_name::#ident { #( #names ),* } => #name::#ident { #( #names_3: ::specs::saveload::FromDeserialize::from(#names_2, &mut ids)? ),* },
                };
            }
            Fields::Unnamed(ref fields) => {
                let field_ids: Vec<_> = fields
                    .unnamed
                    .iter()
                    .cloned()
                    .enumerate()
                    .map(|(i, _)| Some(Ident::new(&format!("field{}", i), data.enum_token.0)))
                    .collect();

                let field_ids_2 = field_ids.clone();

                big_match_ser = quote!{
                    #big_match_ser
                    #name::#ident( #( ref #field_ids ),* ) => #saveload_name::#ident( #( ::specs::saveload::IntoSerialize::into(#field_ids_2, &mut ids)? ),* ),
                };

                let field_ids: Vec<_> = fields
                    .unnamed
                    .iter()
                    .cloned()
                    .enumerate()
                    .map(|(i, _)| Some(Ident::new(&format!("field{}", i), data.enum_token.0)))
                    .collect();

                let field_ids_2 = field_ids.clone();

                big_match_de = quote!{
                    #big_match_de
                    #saveload_name::#ident( #( #field_ids ),* ) => #name::#ident( #( ::specs::saveload::FromDeserialize::from(#field_ids_2, &mut ids)? ),* ),
                };
            }
            Fields::Unit => {
                big_match_ser = quote! {
                    #big_match_ser
                    #name::#ident => #saveload_name::#ident,
                };

                big_match_de = quote! {
                    #big_match_de
                    #saveload_name::#ident => #name::#ident,
                };
            }
        }
    }

    let ser = quote!{
        Ok(match *self {
            #big_match_ser
        })
    };

    let de = quote!{
        Ok(match data {
            #big_match_de
        })
    };

    SaveloadDerive {
        type_def: enum_def,
        ser,
        de,
        saveload_name,
        saveload_generics: saveload_generics.clone(), // dumb borrow checker, don't feel like fixing because it's probably NBD
    }
}

// Edits the bounds of the input type to ensure all fields have `bound`.
fn add_bound(generics: &mut Generics, bound: TypeParamBound) {
    use syn::GenericParam;
    for param in &mut generics.params {
        if let GenericParam::Type(ref mut type_param) = *param {
            if type_param.bounds.iter().position(|b| bound == *b).is_some() {
                continue;
            }
            type_param.bounds.push(bound.clone());
        }
    }
}

// Replaces the type with its corresponding `Data` type.
fn replace_entity_type(ty: &mut Type) {
    match *ty {
        Type::Array(ref mut ty) => replace_entity_type(&mut *ty.elem),
        Type::Tuple(ref mut ty) => {
            for ty in ty.elems.iter_mut() {
                replace_entity_type(&mut *ty);
            }
        }
        Type::Paren(ref mut ty) => replace_entity_type(&mut *ty.elem),
        Type::Path(ref mut ty) => {
            let ty_tok = ty.clone();
            *ty = parse_quote!(<#ty_tok as ::specs::saveload::IntoSerialize<MA>>::Data);
        }
        Type::Group(ref mut ty) => replace_entity_type(&mut *ty.elem),

        Type::TraitObject(_) => {}
        Type::ImplTrait(_) => {}
        Type::Slice(_) => {
            panic!("Slices are unsupported, use owned types like Vecs or Arrays instead")
        }
        Type::Reference(_) => panic!("References are unsupported"),
        Type::Ptr(_) => panic!("Raw pointer types are unsupported"),
        Type::BareFn(_) => panic!("Function types are unsupported"),
        /* We're in a struct so it doesn't matter */
        Type::Never(_) => unreachable!(),
        Type::Infer(_) => unreachable!(),
        Type::Macro(_) => unreachable!(),
        Type::Verbatim(_) => unimplemented!(),
    }
}
