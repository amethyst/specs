//! Contains implementations for `#[derive(Saveload)]`
//! which derives `ConvertSaveload`.

// NOTE: All examples given in the documentation below are "cleaned up" into readable Rust,
// so it doesn't give an entirely accurate view of what's actually generated.

use proc_macro2::{Span, TokenStream};
use syn::{
    DataEnum, DataStruct, DeriveInput, Field, GenericParam, Generics, Ident, Type, WhereClause,
    WherePredicate,
};

/// Handy collection since tuples got unwieldy and
/// unclear in purpose
struct SaveloadDerive {
    type_def: TokenStream,
    ser: TokenStream,
    de: TokenStream,
    saveload_name: Ident,
}

/// The main entrypoint, sets things up and delegates to the
/// type we're deriving on.
pub fn impl_saveload(ast: &mut DeriveInput) -> TokenStream {
    use syn::Data;

    add_where_clauses(
        &mut ast.generics.where_clause,
        &ast.generics.params,
        |ty| parse_quote!(#ty: ConvertSaveload<MA, Error = NoError> + ConvertSaveload<MA, Error = NoError>),
    );
    add_where_clauses(
        &mut ast.generics.where_clause,
        &ast.generics.params,
        |ty| parse_quote!(<#ty as ConvertSaveload<MA>>::Data: ::serde::Serialize + ::serde::de::DeserializeOwned + Clone),
    );
    add_where_clauses(
        &mut ast.generics.where_clause,
        &ast.generics.params,
        |ty| parse_quote!(<#ty as ConvertSaveload<MA>>::Data: ::serde::Serialize + ::serde::de::DeserializeOwned + Clone),
    );
    add_where_clause(
        &mut ast.generics.where_clause,
        parse_quote!(MA: ::serde::Serialize + ::serde::de::DeserializeOwned + Marker),
    );
    add_where_clause(
        &mut ast.generics.where_clause,
        parse_quote!(for <'deser> MA: ::serde::Deserialize<'deser>),
    );

    let derive = match ast.data {
        Data::Struct(ref mut data) => saveload_struct(data, &mut ast.ident, &mut ast.generics),
        Data::Enum(ref data) => saveload_enum(data, &ast.ident, &ast.generics),
        Data::Union(_) => panic!("Unions cannot derive `ConvertSaveload`"),
    };

    let name = &ast.ident;

    let mut impl_generics = ast.generics.clone();
    impl_generics.params.push(parse_quote!(MA));
    let (impl_generics, saveload_ty_generics, where_clause) = impl_generics.split_for_impl();
    let (_, ty_generics, _) = ast.generics.split_for_impl(); // We don't want the type generics we just made
                                                             // because they have MA which our normal type doesn't have

    let type_def = derive.type_def;
    let saveload_name = derive.saveload_name;

    let ser = derive.ser;
    let de = derive.de;

    let tt = quote! {
        #[derive(Serialize, Deserialize, Clone)]
        #[serde(bound = "MA: Marker")]
        pub #type_def

        impl #impl_generics ConvertSaveload<MA> for #name #ty_generics #where_clause {
            type Data = #saveload_name #saveload_ty_generics;
            type Error = NoError;

            fn convert_into<F>(&self, mut ids: F) -> Result<Self::Data, Self::Error>
            where
                F: FnMut(Entity) -> Option<MA>
            {
                #ser
            }

            fn convert_from<F>(data: Self::Data, mut ids: F) -> Result<Self, Self::Error>
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

/// Implements all elements of saveload common to structs of any type
fn saveload_struct(
    data: &mut DataStruct,
    name: &mut Ident,
    generics: &mut Generics,
) -> SaveloadDerive {
    use syn::Fields;

    let mut saveload_generics = generics.clone();
    saveload_generics.params.push(parse_quote!(MA));

    let saveload_name = Ident::new(&format!("{}SaveloadData", name), Span::call_site());

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
        panic!("Every unit struct `Serialize`/`Deserialize` thus blanket impl already implements `ConvertSaveload` for them.");
    };

    SaveloadDerive {
        type_def: struct_def,
        ser,
        de,
        saveload_name,
    }
}

/// Automatically derives the two traits and proxy `Data` container for a struct with named fields (e.g. `struct Foo {e: Entity}`).
///
/// This generates a struct named `StructNameSaveloadData` such that all fields are their associated `Data` variants, as well as a bound on the required marker
///  e.g.
///
/// ```nobuild
/// struct FooSaveloadData<MA> where MA: Serialize+Marker, for<'de> MA: Deserialize<'de> {
///    e: <Entity as ConvertSaveload<MA>>::Data
/// }
/// ```
///
/// The generation for the `into` and `from` functions just constructs each field by calling `into`/`from` for each field in the input struct
/// and placing it into the output struct:
///
/// ```nobuild
///  fn into<F: FnMut(Entity) -> Option<MA>>(&self, mut ids: F) -> Result<Self::Data, Self::Error> {
///      FooSaveloadData {
///          e: ConvertSaveload::convert_into(&self.e, &mut ids)?
///      }
///  }
/// ```
fn saveload_named_struct(
    name: &Ident,
    saveload_name: &Ident,
    generics: &Generics,
    saveload_fields: &[Field],
) -> (TokenStream, TokenStream, TokenStream) {
    let (_, ty_generics, where_clause) = generics.split_for_impl();

    let struct_def = quote! {
        struct #saveload_name #ty_generics #where_clause {
            #( #saveload_fields ),*
        }
    };

    let field_names = saveload_fields.iter().map(|f| f.ident.clone());
    let field_names_2 = field_names.clone();
    let tmp = field_names.clone();
    let ser = quote! {
        Ok(#saveload_name {
            # ( #field_names: ConvertSaveload::convert_into(&self.#field_names_2, &mut ids)? ),*
        })
    };

    let field_names = tmp;
    let field_names_2 = field_names.clone();
    let de = quote! {
        Ok(#name {
            # ( #field_names: ConvertSaveload::convert_from(data.#field_names_2, &mut ids)? ),*
        })
    };

    (struct_def, ser, de)
}

/// Automatically derives the two traits and proxy `Data` container for a struct with unnamed fields aka a tuple struct (e.g. `struct Foo(Entity);`).
///
/// This generates a struct named `StructNameSaveloadData` such that all fields are their associated `Data` variants, as well as a bound on the required marker
///  e.g.
///
/// ```nobuild
/// struct FooSaveloadData<MA> (
///    <Entity as ConvertSaveload<MA>>::Data
/// ) where MA: Serialize+Marker, for<'de> MA: Deserialize<'de>;
/// ```
///
/// The generation for the `into` and `from` functions just constructs each field by calling `into`/`from` for each field in the input struct
/// and placing it into the output struct:
///
/// ```nobuild
///  fn into<F: FnMut(Entity) -> Option<MA>>(&self, mut ids: F) -> Result<Self::Data, Self::Error> {
///      FooSaveloadData (
///          ConvertSaveload::convert_into(&self.0, &mut ids)?
///      )
///  }
/// ```
fn saveload_tuple_struct(
    data: &DataStruct,
    name: &Ident,
    saveload_name: &Ident,
    generics: &Generics,
    saveload_fields: &[Field],
) -> (TokenStream, TokenStream, TokenStream) {
    use syn::Index;

    let (_, ty_generics, where_clause) = generics.split_for_impl();

    let struct_def = quote! {
        struct #saveload_name #ty_generics (
            #( #saveload_fields ),*
        ) #where_clause;
    };

    let field_ids = saveload_fields.iter().enumerate().map(|(i, _)| Index {
        index: i as u32,
        span: data.struct_token.span.clone(),
    });
    let tmp = field_ids.clone();
    let ser = quote! {
        Ok(#saveload_name (
            # ( ConvertSaveload::convert_into(&self.#field_ids, &mut ids)? ),*
        ))
    };

    let field_ids = tmp;
    let de = quote! {
        Ok(#name (
            # ( ConvertSaveload::convert_from(data.#field_ids, &mut ids)? ),*
        ))
    };

    (struct_def, ser, de)
}

/// Automatically derives the two traits and proxy `Data` container for an Enum (e.g. `enum Foo{ Bar(Entity), Baz{ e: Entity }, Unit }`).
///
/// This will properly handle enum variants with no `Entity`, so long as at least one variant (or one of that variant's fields recursively) contain an
/// `Entity` somewhere. If this isn't true, `Saveload` is auto-derived so long as the fields can be `Serialize` and `Deserialize` anyway.
///
/// This generates a struct named `EnumNameSaveloadData` such that all fields are their associated `Data` variants, as well as a bound on the required marker
///  e.g.
///
/// ```nobuild
/// enum FooSaveloadData<MA> where MA: Serialize+Marker, for<'de> MA: Deserialize<'de> {
///    Bar(<Entity as ConvertSaveload<MA>>::Data),
///    Baz { e: <Entity as ConvertSaveload<MA>>::Data },
///    Unit
/// };
/// ```
///
/// The generation for the `into` and `from` functions just constructs each field of each variant by calling `into`/`from` for each field in the input
/// and placing it into the output struct in a giant match of each possibility:
///
/// ```nobuild
///  fn into<F: FnMut(Entity) -> Option<MA>>(&self, mut ids: F) -> Result<Self::Data, Self::Error> {
///      match *self {
///          Foo::Bar(ref field0) => FooSaveloadData::Bar(ConvertSaveload::convert_into(field0, &mut ids)? ),
///          Foo::Baz{ ref e } => FooSaveloadData::Baz{ e: ConvertSaveload::convert_into(e, &mut ids)? },
///          Foo::Unit => FooSaveloadData::Unit,
///      }
///  }
/// ```
fn saveload_enum(data: &DataEnum, name: &Ident, generics: &Generics) -> SaveloadDerive {
    use syn::Fields;

    let mut saveload_generics = generics.clone();
    saveload_generics.params.push(parse_quote!(MA));

    let saveload_name = Ident::new(&format!("{}SaveloadData", name), Span::call_site());

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
    let enum_def = quote! {
        enum #saveload_name #saveload_ty_generics #saveload_where_clause {
            #( #variants ),*
        }
    };

    let mut big_match_ser = quote! {};
    let mut big_match_de = quote! {};

    for variant in variants {
        let ident = &variant.ident;

        match &variant.fields {
            Fields::Named(fields) => {
                let get_names = || fields.named.iter().map(|f| f.ident.clone());
                let names = get_names();
                let names_2 = get_names();
                let names_3 = get_names();

                big_match_ser = quote! {
                    #big_match_ser
                    #name::#ident { #( ref #names ),* } => #saveload_name::#ident { #( #names_3: ConvertSaveload::convert_into(#names_2, ids)? ),* },
                };

                let names = get_names();
                let names_2 = get_names();
                let names_3 = get_names();

                big_match_de = quote! {
                    #big_match_de
                    #saveload_name::#ident { #( #names ),* } => #name::#ident { #( #names_3: ConvertSaveload::convert_from(#names_2, &mut ids)? ),* },
                };
            }
            Fields::Unnamed(fields) => {
                let field_ids: Vec<_> = fields
                    .unnamed
                    .iter()
                    .cloned()
                    .enumerate()
                    .map(|(i, _)| Some(Ident::new(&format!("field{}", i), data.enum_token.span)))
                    .collect();
                let field_ids_2 = field_ids.clone();
                let tmp = field_ids.clone();

                big_match_ser = quote! {
                    #big_match_ser
                    #name::#ident( #( ref #field_ids ),* ) => #saveload_name::#ident( #( ConvertSaveload::convert_into(#field_ids_2, &mut ids)? ),* ),
                };

                let field_ids = tmp;
                let field_ids_2 = field_ids.clone();

                big_match_de = quote! {
                    #big_match_de
                    #saveload_name::#ident( #( #field_ids ),* ) => #name::#ident( #( ConvertSaveload::convert_from(#field_ids_2, &mut ids)? ),* ),
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

    let ser = quote! {
        Ok(match *self {
            #big_match_ser
        })
    };

    let de = quote! {
        Ok(match data {
            #big_match_de
        })
    };

    SaveloadDerive {
        type_def: enum_def,
        ser,
        de,
        saveload_name,
    }
}

/// Adds where clause for each type parameter
fn add_where_clauses<'a, F, I>(where_clause: &mut Option<WhereClause>, generics: I, mut clause: F)
where
    F: FnMut(Ident) -> WherePredicate,
    I: IntoIterator<Item = &'a GenericParam>,
{
    use syn::GenericParam;
    let preds = &mut where_clause.get_or_insert(parse_quote!(where)).predicates;
    for generic in generics {
        if let GenericParam::Type(ty_param) = generic {
            let ty_param = ty_param.ident.clone();
            preds.push(clause(ty_param));
        }
    }
}

/// Adds where clause
fn add_where_clause(where_clause: &mut Option<WhereClause>, pred: WherePredicate) {
    where_clause
        .get_or_insert(parse_quote!(where))
        .predicates
        .push(pred);
}

/// Replaces the type with its corresponding `Data` type.
fn replace_entity_type(ty: &mut Type) {
    match ty {
        Type::Array(ty) => replace_entity_type(&mut *ty.elem),
        Type::Tuple(ty) => {
            for ty in ty.elems.iter_mut() {
                replace_entity_type(&mut *ty);
            }
        }
        Type::Paren(ty) => replace_entity_type(&mut *ty.elem),
        Type::Path(ty) => {
            let ty_tok = ty.clone();
            *ty = parse_quote!(<#ty_tok as ConvertSaveload<MA>>::Data);
        }
        Type::Group(ty) => replace_entity_type(&mut *ty.elem),

        Type::TraitObject(_) => {}
        Type::ImplTrait(_) => {}
        Type::Slice(_) => {
            panic!("Slices are unsupported, use owned types like Vecs or Arrays instead")
        }
        Type::Reference(_) => panic!("References are unsupported"),
        Type::Ptr(_) => panic!("Raw pointer types are unsupported"),
        Type::BareFn(_) => panic!("Function types are unsupported"),
        Type::Never(_) => panic!("Never type is unsupported"),
        /* We're in a struct so it doesn't matter */
        Type::Infer(_) => unreachable!(),
        Type::Macro(_) => unreachable!(),
        Type::Verbatim(_) => unimplemented!(),
    }
}
