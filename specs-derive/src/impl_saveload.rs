//! Contains implementations for `#[derive(Saveload)]`
//! which derives `ConvertSaveload`.

// NOTE: All examples given in the documentation below are "cleaned up" into
// readable Rust, so it doesn't give an entirely accurate view of what's
// actually generated.

use proc_macro2::{Span, TokenStream};
use syn::{
    Attribute, DataEnum, DataStruct, DeriveInput, Field, GenericParam, Generics, Ident, Type,
    WhereClause, WherePredicate,
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
        |ty| parse_quote!(#ty: ConvertSaveload<MA, Error = std::convert::Infallible> + ConvertSaveload<MA, Error = std::convert::Infallible>),
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
    // We don't want the type generics we just made because they have MA which our
    // normal type doesn't have
    let (_, ty_generics, _) = ast.generics.split_for_impl();

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
            type Error = std::convert::Infallible;

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

struct FieldMetaData {
    field: Field,
    skip_field: bool,
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

    let saveload_fields = convert_fields_to_metadata(&data.fields);

    let (struct_def, ser, de) = if let Fields::Named(_) = data.fields {
        saveload_named_struct(&name, &saveload_name, &saveload_generics, &saveload_fields)
    } else if let Fields::Unnamed(_) = data.fields {
        saveload_tuple_struct(
            data,
            &name,
            &saveload_name,
            &saveload_generics,
            &saveload_fields,
        )
    } else {
        panic!(
            "Every unit struct `Serialize`/`Deserialize` thus blanket impl already implements `ConvertSaveload` for them."
        );
    };

    SaveloadDerive {
        type_def: struct_def,
        ser,
        de,
        saveload_name,
    }
}

/// Automatically derives the two traits and proxy `Data` container for a struct
/// with named fields (e.g. `struct Foo {e: Entity}`).
///
/// This generates a struct named `StructNameSaveloadData` such that all fields
/// are their associated `Data` variants, as well as a bound on the required
/// marker  e.g.
///
/// ```nobuild
/// struct FooSaveloadData<MA> where MA: Serialize+Marker, for<'de> MA: Deserialize<'de> {
///    e: <Entity as ConvertSaveload<MA>>::Data
/// }
/// ```
///
/// The generation for the `into` and `from` functions just constructs each
/// field by calling `into`/`from` for each field in the input struct
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
    saveload_fields: &[FieldMetaData],
) -> (TokenStream, TokenStream, TokenStream) {
    let (_, ty_generics, where_clause) = generics.split_for_impl();

    let fields = saveload_fields.iter().map(|f| &f.field);

    let struct_def = quote! {
        struct #saveload_name #ty_generics #where_clause {
            #( #fields ),*
        }
    };

    let field_ser = saveload_fields.iter().map(|field_meta| {
        let field = &field_meta.field;
        let field_ident = &field.ident;

        if field_meta.skip_field {
            quote! { #field_ident: self.#field_ident.clone() }
        } else {
            quote! { #field_ident: ConvertSaveload::convert_into(&self.#field_ident, &mut ids)? }
        }
    });

    let ser = quote! {
        Ok(#saveload_name {
            #(#field_ser),*
        })
    };

    let field_de = saveload_fields
        .iter()
        .map(|field_meta| {
            let field = &field_meta.field;
            let field_ident = &field.ident;

            if field_meta.skip_field {
                quote! { #field_ident: data.#field_ident }
            } else {
                quote! { #field_ident: ConvertSaveload::convert_from(data.#field_ident, &mut ids)? }
            }
        })
        .collect::<Vec<_>>();

    let de = quote! {
        Ok(#name {
            #(#field_de),*
        })
    };

    (struct_def, ser, de)
}

/// Automatically derives the two traits and proxy `Data` container for a struct
/// with unnamed fields aka a tuple struct (e.g. `struct Foo(Entity);`).
///
/// This generates a struct named `StructNameSaveloadData` such that all fields
/// are their associated `Data` variants, as well as a bound on the required
/// marker  e.g.
///
/// ```nobuild
/// struct FooSaveloadData<MA> (
///    <Entity as ConvertSaveload<MA>>::Data
/// ) where MA: Serialize+Marker, for<'de> MA: Deserialize<'de>;
/// ```
///
/// The generation for the `into` and `from` functions just constructs each
/// field by calling `into`/`from` for each field in the input struct
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
    saveload_fields: &[FieldMetaData],
) -> (TokenStream, TokenStream, TokenStream) {
    use syn::Index;

    let (_, ty_generics, where_clause) = generics.split_for_impl();

    let fields = saveload_fields.iter().map(|f| &f.field);

    let struct_def = quote! {
        struct #saveload_name #ty_generics (
            #( #fields ),*
        ) #where_clause;
    };

    let field_ids = saveload_fields
        .iter()
        .enumerate()
        .map(|(i, _field_meta)| Index {
            index: i as u32,
            span: data.struct_token.span,
        })
        .collect::<Vec<_>>();

    let field_ser = saveload_fields
        .iter()
        .zip(field_ids.iter())
        .map(|(field_meta, field_id)| {
            if field_meta.skip_field {
                quote! { self.#field_id.clone() }
            } else {
                quote! { ConvertSaveload::convert_into(&self.#field_id, &mut ids)? }
            }
        })
        .collect::<Vec<_>>();

    let ser = quote! {
        Ok(#saveload_name (
            #(#field_ser),*
        ))
    };

    let field_de = saveload_fields
        .iter()
        .zip(field_ids)
        .map(|(field_meta, field_id)| {
            if field_meta.skip_field {
                quote! { data.#field_id }
            } else {
                quote! { ConvertSaveload::convert_from(data.#field_id, &mut ids)? }
            }
        })
        .collect::<Vec<_>>();

    let de = quote! {
        Ok(#name (
            #(#field_de),*
        ))
    };

    (struct_def, ser, de)
}

fn convert_fields_to_metadata<'a, T>(fields: T) -> Vec<FieldMetaData>
where
    T: IntoIterator<Item = &'a Field>,
{
    fields
        .into_iter()
        .map(|f| {
            let mut resolved = f.clone();

            replace_field(&mut resolved);

            FieldMetaData {
                field: resolved,
                skip_field: field_should_skip(&f),
            }
        })
        .collect()
}

/// Automatically derives the two traits and proxy `Data` container for an Enum
/// (e.g. `enum Foo{ Bar(Entity), Baz{ e: Entity }, Unit }`).
///
/// This will properly handle enum variants with no `Entity`, so long as at
/// least one variant (or one of that variant's fields recursively) contain an
/// `Entity` somewhere. If this isn't true, `Saveload` is auto-derived so long
/// as the fields can be `Serialize` and `Deserialize` anyway.
///
/// This generates a struct named `EnumNameSaveloadData` such that all fields
/// are their associated `Data` variants, as well as a bound on the required
/// marker  e.g.
///
/// ```nobuild
/// enum FooSaveloadData<MA> where MA: Serialize+Marker, for<'de> MA: Deserialize<'de> {
///    Bar(<Entity as ConvertSaveload<MA>>::Data),
///    Baz { e: <Entity as ConvertSaveload<MA>>::Data },
///    Unit
/// };
/// ```
///
/// The generation for the `into` and `from` functions just constructs each
/// field of each variant by calling `into`/`from` for each field in the input
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
    use syn::{Fields, Variant};

    let mut saveload_generics = generics.clone();
    saveload_generics.params.push(parse_quote!(MA));

    let saveload_name = Ident::new(&format!("{}SaveloadData", name), Span::call_site());

    let saveload_variants: Vec<Variant> = data
        .variants
        .iter()
        .map(|v| {
            let mut saveload_variant = v.clone();

            replace_attributes(&mut saveload_variant.attrs);

            for field in saveload_variant.fields.iter_mut() {
                replace_field(field);
            }

            saveload_variant
        })
        .collect();

    let (_, saveload_ty_generics, saveload_where_clause) = saveload_generics.split_for_impl();
    let enum_def = quote! {
        enum #saveload_name #saveload_ty_generics #saveload_where_clause {
            #( #saveload_variants ),*
        }
    };

    let mut big_match_ser = quote! {};
    let mut big_match_de = quote! {};

    for variant in data.variants.iter() {
        let ident = &variant.ident;

        match &variant.fields {
            Fields::Named(fields) => {
                let saveload_fields = convert_fields_to_metadata(&fields.named);

                let field_ser = saveload_fields.iter().map(|field_meta| {
                    let field_ident = &field_meta.field.ident;

                    if field_meta.skip_field {
                        quote!{ #field_ident: #field_ident.clone() }
                    } else {
                        quote!{ #field_ident: ConvertSaveload::convert_into(#field_ident, &mut ids)? }
                    }
                });

                let names = fields
                    .named
                    .iter()
                    .map(|f| f.ident.clone())
                    .collect::<Vec<_>>();

                big_match_ser = quote! {
                    #big_match_ser
                    #name::#ident { #( ref #names ),* } => #saveload_name::#ident { #(#field_ser),* },
                };

                let field_de = saveload_fields.iter().map(|field_meta| {
                    let field_ident = &field_meta.field.ident;

                    if field_meta.skip_field {
                        quote!{ #field_ident: #field_ident }
                    } else {
                        quote!{ #field_ident: ConvertSaveload::convert_from(#field_ident, &mut ids)? }
                    }
                })
                .collect::<Vec<_>>();

                big_match_de = quote! {
                    #big_match_de
                    #saveload_name::#ident { #( #names ),* } => #name::#ident { #(#field_de),* },
                };
            }
            Fields::Unnamed(fields) => {
                let saveload_fields = convert_fields_to_metadata(&fields.unnamed);

                let field_ids = saveload_fields
                    .iter()
                    .enumerate()
                    .map(|(i, _)| Ident::new(&format!("field{}", i), data.enum_token.span))
                    .collect::<Vec<_>>();

                let field_ser = saveload_fields
                    .iter()
                    .zip(field_ids.iter())
                    .map(|(field_meta, field_ident)| {
                        if field_meta.skip_field {
                            quote! { #field_ident.clone() }
                        } else {
                            quote! { ConvertSaveload::convert_into(#field_ident, &mut ids)? }
                        }
                    })
                    .collect::<Vec<_>>();

                big_match_ser = quote! {
                    #big_match_ser
                    #name::#ident( #( ref #field_ids ),* ) => #saveload_name::#ident( #(#field_ser),* ),
                };

                let field_de = saveload_fields
                    .iter()
                    .zip(field_ids.iter())
                    .map(|(field_meta, field_ident)| {
                        if field_meta.skip_field {
                            quote! { #field_ident.clone() }
                        } else {
                            quote! { ConvertSaveload::convert_from(#field_ident, &mut ids)? }
                        }
                    })
                    .collect::<Vec<_>>();

                big_match_de = quote! {
                    #big_match_de
                    #saveload_name::#ident( #( #field_ids ),* ) => #name::#ident( #(#field_de),* ),
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

fn attribute_is_skip(attribute: &Attribute) -> bool {
    attribute.path.is_ident("convert_save_load_skip_convert")
}

fn field_should_skip(field: &Field) -> bool {
    field.attrs.iter().any(attribute_is_skip)
}

fn replace_field(field: &mut Field) {
    if !field_should_skip(field) {
        replace_entity_type(&mut field.ty);
    }

    replace_attributes(&mut field.attrs);
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
        _ => unimplemented!(),
    }
}

fn single_parse_outer_from_args(input: syn::parse::ParseStream) -> Result<Attribute, syn::Error> {
    Ok(Attribute {
        pound_token: syn::token::Pound {
            spans: [input.span()],
        },
        style: syn::AttrStyle::Outer,
        bracket_token: syn::token::Bracket { span: input.span() },
        path: input.call(syn::Path::parse_mod_style)?,
        tokens: input.parse()?,
    })
}

fn replace_attributes(attrs: &mut Vec<Attribute>) {
    let output_attrs = attrs
        .iter()
        .filter_map(|attr| {
            if attr.path.is_ident("convert_save_load_skip_convert") {
                None
            } else if attr.path.is_ident("convert_save_load_attr") {
                match attr.parse_args_with(single_parse_outer_from_args) {
                    Ok(inner_attr) => Some(inner_attr),
                    Err(err) => panic!(
                        "Unparseable inner attribute on convert_save_load_attr: {}",
                        err
                    ),
                }
            } else {
                Some(attr.clone())
            }
        })
        .collect::<Vec<_>>();

    *attrs = output_attrs
}
