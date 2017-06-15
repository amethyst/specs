
use std::iter::{FromIterator, IntoIterator};
use std::ops::{Deref, DerefMut};
use std::marker::PhantomData;
use std::any::Any;

use syn::{parse, Ident, VariantData, Attribute, NestedMetaItem, MetaItem, Body, DeriveInput, Field, Ty};
use quote::{ToTokens, Tokens};

use specs::entity::Split;

/// Expands the `ComponentGroup` derive implementation.
#[allow(unused_variables)]
pub fn expand_group(input: &DeriveInput) -> Result<Tokens, String> {
    let name = &input.ident;
    let items = ItemList::new(input);

    let dummy_const = Ident::new(format!("_IMPL_COMPONENTGROUP_FOR_{}", name));
    let macro_const = Ident::new(format!("call_{}", name));
    let macro_const_call = Ident::new(format!("{}!", macro_const));

    // Duplicate references to the components.
    // `quote` currently doesn't support using the same variable binding twice in a repetition.

    let items = items.into_iter()
        .filter(|item| !item.parameter.ignore )
        .collect::<ItemList>();

    // Local component fields
    let ref component = items.iter()
        .filter(|item| !item.parameter.subgroup )
        .collect::<ItemList>();
    let component2 = component;
    let component3 = component;

    // Serializable components
    let ref component_serialize = component.iter()
        .filter(|item| item.parameter.serialize)
        .collect::<ItemList>();
    let component_serialize2 = component_serialize;

    // Serializable component names
    let ref component_serialize_field = component_serialize.iter()
        .map(|item| item.field.ident.as_ref().unwrap() )
        .collect::<Vec<&Ident>>();

    // Subgroup fields
    let ref subgroup = items.iter()
        .filter(|item| item.parameter.subgroup )
        .collect::<ItemList>();
    
    // Subgroup macro names
    let ref subgroup_macro = subgroup.iter()
        .map(|item| {
            let mut tokens = Tokens::new();
            item.field.ty.to_tokens(&mut tokens);
            Ident::new(format!("call_{}!", tokens.as_str()))
        })
        .collect::<Vec<_>>();

    // Get all components from a subgroup
    let local_count = component.iter().count();
    let subgroup_count = subgroup.iter().count();

    // Construct Split tuple groups for the components and subgroups.
    let local_ty = component.type_list();
    let subgroup_ty = subgroup.type_list();

    let locals_impl = quote! {
        impl _specs::entity::DeconstructedGroup for #name {
            type All = SplitConcat<
                #local_ty,
                #(
                    <#subgroup as DeconstructedGroup>::All,
                )*
                #subgroup_ty
            >;
            type Locals = #local_ty;
            type Subgroups = #subgroup_ty;
            fn locals() -> usize { #local_count }
            fn subgroups() -> usize { #subgroup_count }
            fn all() -> usize {
                let mut count = #local_count;
                #( count += <#subgroup as DeconstructedGroup>::all(); )*
                count
            }
        }
    };

    // Macro for expanding usage... 
    let expanded_macro = quote! {
        #[allow(unused_macros)]
        macro_rules! #macro_const {
            ($($option:ident):* =>
                fn $($method:ident).*
                [$( $before:ty ),*] in [$( $after:ty ),*]
                ($( $args:expr ),*)
            ) => {{
                #macro_const_call($($option):* => IMPL
                    fn $($method).*
                    [$( $before ),*] in [$( $after ),*]
                    ( $( $args ),* )
                );
            }};
            // all components and subgroups
            (all => IMPL 
                fn $($method:ident).*
                [$( $before:ty ),*] in [$( $after:ty ),*]
                ($( $args:expr ),*)
            ) => {{
                let result =
                (#(
                    $($method).*::<$( $before, )* #component $(, $after)* >( $( $args ),* ),
                )*);

                let result = (result, (#(
                    #subgroup_macro(all => IMPL
                        fn $($method).*
                        [$( $before ),*] in [$( $after ),*]
                        ( $( $args ),* )
                    ),
                )*));

                result
            }};
            // only components in this subgroup
            (local => IMPL
                fn $($method:ident).*
                [$( $before:ty ),*] in [$( $after:ty ),*]
                ($( $args:expr ),*)
            ) => {{
                let result =
                (#(
                    $($method).*::<$( $before, )* #component $(, $after)* >( $( $args ),* ),
                )*);

                result
            }};
            // only subgroups in this subgroup (not useful on its own but useful together with local components)
            (subgroups => IMPL
                fn $($method:ident).*
                [$( $before:ty ),*] in [$( $after:ty ),*]
                ($( $args:expr ),*)
            ) => {{
                let result =
                (#(
                    $($method).*::<$( $before, )* #subgroup $(, $after)* >( $( $args ),* ),
                )*);

                result
            }};
        };
    };

    // Normal group methods.
    let default = quote! {
        fn local_components() -> Vec<&'static str> {
            vec![ #( stringify!(#component) ),* ]
        }
        #[allow(unused_mut)]
        fn components() -> Vec<&'static str> {
            let mut list = #name::local_components();
            #(
                for component in #subgroup::components() {
                    list.push(component);
                }
            )*
            list
        }
        fn subgroups() -> Vec<&'static str> {
            vec![ #( stringify!(#subgroup), )* ]
        }
        fn register(world: &mut _specs::World) {
            #(
                world.register::<#component>();
            )*

            #(
                #subgroup::register(world);
            )*
        }
    };

    // Serialization methods
    #[cfg(feature="serialize")]
    let serialize = quote! {
        fn serialize_group<S: _serde::Serializer>(world: &_specs::World, serializer: S) -> Result<S::Ok, S::Error> {
            let mut map = serializer.serialize_map(None)?;
            #(
                let storage = world.read::<#component_serialize>();
                _serde::ser::SerializeMap::serialize_entry(&mut map, stringify!(#component_serialize_field), &storage)?;
            )*

            #(
                #subgroup::serialize_subgroup::<S>(world, &mut map)?;
            )*

            _serde::ser::SerializeMap::end(map)
        }

        fn serialize_subgroup<S: _serde::Serializer>(world: &_specs::World, map: &mut S::SerializeMap) -> Result<(), S::Error> {
            #(
                let storage = world.read::<#component_serialize>();
                _serde::ser::SerializeMap::serialize_entry(map, stringify!(#component_serialize_field), &storage)?;
            )*

            #(
                #subgroup::serialize_subgroup::<S>(world, map)?;
            )*

            Ok(())
        }

        fn deserialize_group<D: _serde::Deserializer>(world: &mut _specs::World, entities: &[_specs::Entity], deserializer: D) -> Result<(), D::Error> {
            use std::fmt;

            struct ComponentVisitor<'a>(&'a mut _specs::World, &'a [_specs::Entity]);
            impl<'a> _serde::de::Visitor for ComponentVisitor<'a> {
                type Value = ();
                fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
                    write!(formatter, "a map of component identifiers to packed data")
                }

                fn visit_map<V>(self, mut visitor: V) -> Result<(), V::Error>
                    where V: _serde::de::MapVisitor
                {
                    while let Some(key) = visitor.visit_key::<String>()? {
                        #[allow(unused_variables)]
                        match key {
                            #(
                                #component_serialize_field => {
                                    let mut storage = self.0.write::<#component_serialize>();
                                    let packed = visitor.visit_value::<_specs::PackedData<#component_serialize2>>()?;
                                    let _ = storage.merge(self.1, packed);
                                },
                            )*
                            key @ _ => {
                                #(
                                    if let Some(()) = <#subgroup as _specs::entity::SerializeGroup>::deserialize_subgroup(self.0, self.1, key.to_owned(), &mut visitor)? {
                                        continue; // subgroup deserialized the components
                                    }
                                )*
                                continue; // not in the registered component list, ignore
                            },
                        }
                    }

                    Ok(())
                }
            }

            Ok(deserializer.deserialize_map(ComponentVisitor(world, entities))?)
        }

        fn deserialize_subgroup<V>(world: &mut _specs::World, entities: &[_specs::Entity], key: String, mut visitor: &mut V) -> Result<Option<()>, V::Error>
            where V: _serde::de::MapVisitor
        {
            #[allow(unused_variables)]
            match key {
                #(
                    #component_serialize_field => {
                        let mut storage = world.write::<#component_serialize>();
                        let packed = visitor.visit_value::<_specs::PackedData<#component_serialize2>>()?;
                        let _ = storage.merge(entities, packed);
                        Ok(Some(()))
                    },
                )*
                key @ _ => {
                    #(
                        if let Some(()) = #subgroup::deserialize_subgroup(world, entities, key.to_owned(), visitor)? {
                            return Ok(Some(()));
                        }
                    )*
                    Ok(None)
                },
            }
        }
    };

    // Normal expand (no serialization)
    let expanded = quote! {
        #[automatically_derived]
        impl _specs::entity::ComponentGroup for #name {
            #default
        }
    };

    // Serialization expand
    #[cfg(feature="serialize")]
    let expanded = quote! {
        extern crate serde as _serde;
        #expanded
        impl _specs::entity::SerializeGroup for #name {
            #serialize
        }
    };

    // Wrap the expanded code to prevent context conflicts.
    let wrap = quote! {
        #[allow(non_upper_case_globals, unused_attributes, unused_qualifications)]
        #[macro_use]
        const #dummy_const: () = {
            extern crate specs as _specs;
            #locals_impl
            #expanded
        };
    };

    Ok(wrap)
}

#[derive(Debug, Clone)]
pub struct Parameter {
    // Is a subgroup.
    pub subgroup: bool,

    // Serialize this field
    pub serialize: bool,

    // Ignore this field completely
    pub ignore: bool,

    // Name of item.
    pub name: String,
}

impl Parameter {
    pub fn parse(field: &Field) -> Parameter {
        use syn::NestedMetaItem::MetaItem;
        use syn::MetaItem::Word;

        let mut subgroup = false;
        let mut serialize = false;
        let mut ignore = false;

        for meta_items in field.attrs.iter().filter_map(filter_group) {
            for meta_item in meta_items {
                match meta_item {
                    MetaItem(Word(ref name)) if name == "serialize" => serialize = true,
                    MetaItem(Word(ref name)) if name == "subgroup" => subgroup = true,
                    MetaItem(Word(ref name)) if name == "ignore" => ignore = true,
                    _ => println!("Unused attribute: {:?}", meta_item),
                }
            }
        }

        Parameter {
            subgroup: subgroup,
            serialize: serialize,
            ignore: ignore,
            name: field.ident.clone().unwrap().to_string(),
        }
    }
}

fn filter_group(attr: &Attribute) -> Option<Vec<NestedMetaItem>> {
    match attr.value {
        MetaItem::List(ref name, ref items) if name == "group" => Some(items.iter().cloned().collect()),
        _ => None,
    }
}

#[derive(Debug, Clone)]
struct Item<'a> {
    pub parameter: Parameter,
    pub field: &'a Field,
}

impl<'a> ToTokens for Item<'a> {
    fn to_tokens(&self, tokens: &mut Tokens) {
        self.field.ty.to_tokens(tokens);
    }
}

#[derive(Debug, Clone)]
struct ItemList<'a>(Vec<Item<'a>>);

impl<'a> Deref for ItemList<'a> {
    type Target = Vec<Item<'a>>;
    fn deref(&self) -> &Vec<Item<'a>> {
        &self.0
    }
}

impl<'a> DerefMut for ItemList<'a> {
    fn deref_mut(&mut self) -> &mut Vec<Item<'a>> {
        &mut self.0
    }
}

impl<'a> IntoIterator for ItemList<'a> {
    type Item = Item<'a>;
    type IntoIter = <Vec<Item<'a>> as IntoIterator>::IntoIter;
    fn into_iter(self) -> Self::IntoIter {
        self.0.into_iter()
    }
}

impl<'a: 'b, 'b> IntoIterator for &'b ItemList<'a> {
    type Item = &'b Item<'a>;
    type IntoIter = <&'b Vec<Item<'a>> as IntoIterator>::IntoIter;
    fn into_iter(self) -> Self::IntoIter {
        (&self.0).into_iter()
    }
}

impl<'a> FromIterator<Item<'a>> for ItemList<'a> {
    fn from_iter<I: IntoIterator<Item = Item<'a>>>(iter: I) -> ItemList<'a> {
        let list = <Vec<Item<'a>> as FromIterator<Item<'a>>>::from_iter(iter);
        ItemList::from_list(list)
    }
}

impl<'a: 'b, 'b> FromIterator<&'b Item<'a>> for ItemList<'a> {
    fn from_iter<I: IntoIterator<Item = &'b Item<'a>>>(iter: I) -> ItemList<'a> {
        let list = iter.into_iter().cloned().collect::<Vec<Item<'a>>>();
        ItemList::from_list(list)
    }
}

impl<'a> ItemList<'a> {
    fn new<'b>(input: &'b DeriveInput) -> ItemList<'b> {
        let fields = match input.body {
            Body::Enum(_) => panic!("Enum cannot be a component group"),
            Body::Struct(ref data) => match data {
                &VariantData::Struct(ref fields) => fields,
                _ => panic!("Struct must have named fields"),
            },
        };

        fields
            .iter()
            .map(move |field| {
                Item {
                    parameter: Parameter::parse(&field),
                    field: field,
                }
            })
            .collect::<ItemList>()
    }

    fn from_list<'b>(list: Vec<Item<'b>>) -> ItemList<'b> {
        ItemList(list)
    }

    fn type_list(&self) -> Ty {
        self.0.iter().rev().enumerate().fold(Ty::Tup(Vec::new()), |tup, (index, item)| {
            if index == 0 {
                Ty::Tup(vec![
                    item.field.ty.clone(),
                ])
            }
            else {
                Ty::Tup(vec![
                    item.field.ty.clone(),
                    tup,
                ])
            }
        })
    }
}



/*
struct SplitIter<S>
    where S: Split,
{
    inner: Option<Box<SplitIter<<S as Split>::Next>>>,
    ty: String,
    phantom: PhantomData<S>
}

impl<S> Iterator for SplitIter<S>
    where S: Split,
{
    type Item = String;
    fn next(&mut self) -> Option<String> {
        if let Some(inner) = self.inner {
            inner.next()
        }
        else {
            if <S as Split>::next() {
                self.inner = Some(Box::new(SplitIter::<<S as Split>::Next> {
                    inner: None,
                    ty: next(self.ty),
                    phantom: PhantomData
                }));
                Some(this(self.ty))
            }
            else {
                None
            }
        }
    }
}

struct SplitIter(Box<SplitNext>, String);

impl Iterator for SplitIter {
    type Item = String;
    fn next(&mut self) -> Option<String> {
        if self.0.next {
            self.0 = Box::new(Next::<<<self.0 as SplitNext>::Upper as Split>::Next>(PhantomData));

        }
    }
}
*/
