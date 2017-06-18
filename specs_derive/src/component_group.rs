
use std::iter::{FromIterator, IntoIterator};
use std::ops::{Deref, DerefMut};

use syn::{Ident, VariantData, Attribute, NestedMetaItem, MetaItem, Body, DeriveInput, Field, Ty, Lit};
use quote::Tokens;

/// Expands the `ComponentGroup` derive implementation.
pub fn expand_group(input: &DeriveInput) -> Result<Tokens, String> {
    let name = &input.ident;
    let items = ItemList::new(input);

    let dummy_const = Ident::new(format!("_IMPL_COMPONENTGROUP_FOR_{}", name));

    // Duplicate references to the components.
    // `quote` currently doesn't support using the same variable binding twice in a repetition.

    let items = items.into_iter()
        .filter(|item| !item.parameter.ignore )
        .collect::<ItemList>();

    // Local component fields
    let ref component = items.iter()
        .filter(|item| !item.parameter.subgroup )
        .collect::<ItemList>();
    let ref comp_ty = component.types();

    // Serializable components
    let ref component_serialize = component.iter()
        .filter(|item| item.parameter.serialize)
        .collect::<ItemList>();
    #[allow(unused_variables)]
    let ref comp_serialize_name = component_serialize.names();
    let ref comp_serialize_id = component_serialize.ids();
    let ref comp_serialize_ty = component_serialize.types();
    #[allow(unused_variables)]
    let comp_serialize_ty2 = comp_serialize_ty;

    // Subgroup fields
    let ref subgroup = items.iter()
        .filter(|item| item.parameter.subgroup )
        .collect::<ItemList>();
    let ref subgroup_ty = subgroup.types();

    // Get all components from a subgroup
    let local_count = component.iter().count();
    let subgroup_count = subgroup.iter().count();

    // Construct `Split` tuple groups for the components and subgroups.
    let local_ty = component.type_list();
    let sub_ty = subgroup.type_list();

    // Implementation for deconstructed group for the call macro to work.
    let deconstructed = quote! {
        impl _specs::DeconstructedGroup for #name {
            type Locals = #local_ty;
            type Subgroups = #sub_ty;
            fn locals() -> usize { #local_count }
            fn subgroups() -> usize { #subgroup_count }
        }
    };

    // Normal group methods.
    let default = quote! {
        fn local_components() -> Vec<&'static str> {
            vec![ #( stringify!(#comp_ty), )* ]
        }
        fn components() -> Vec<&'static str> {
            #[allow(unused_mut)]
            let mut list = #name::local_components();
            #(
                for component in #subgroup_ty::components() {
                    list.push(component);
                }
            )*
            list
        }
        fn subgroups() -> Vec<&'static str> {
            vec![ #( stringify!(#subgroup_ty), )* ]
        }
        fn register(world: &mut _specs::World) {
            #( world.register::<#comp_ty>(); )*
            #( #subgroup_ty::register(world); )*
        }
    };

    // Serialization methods
    #[cfg(feature="serialize")]
    let serialize = quote! {
        fn serialize_group<S: _serde::Serializer>(world: &_specs::World, serializer: S) -> Result<S::Ok, S::Error> {
            let mut map = serializer.serialize_map(None)?;
            #(
                let storage = world.read_with_id::<#comp_serialize_ty>(#comp_serialize_id);
                _serde::ser::SerializeMap::serialize_entry(&mut map, #comp_serialize_name, &storage)?;
            )*

            #( #subgroup_ty::serialize_subgroup::<S>(world, &mut map)?; )*
            _serde::ser::SerializeMap::end(map)
        }

        fn serialize_subgroup<S: _serde::Serializer>(world: &_specs::World, map: &mut S::SerializeMap) -> Result<(), S::Error> {
            #(
                let storage = world.read_with_id::<#comp_serialize_ty>(#comp_serialize_id);
                _serde::ser::SerializeMap::serialize_entry(map, #comp_serialize_name, &storage)?;
            )*
            #( #subgroup_ty::serialize_subgroup::<S>(world, map)?; )*
            Ok(())
        }

        fn deserialize_group<'de, D: _serde::Deserializer<'de>>(world: &mut _specs::World, entities: &[_specs::Entity], deserializer: D) -> Result<(), D::Error> {
            use std::fmt;

            struct ComponentVisitor<'a>(&'a mut _specs::World, &'a [_specs::Entity]);
            impl<'a, 'de> _serde::de::Visitor<'de> for ComponentVisitor<'a> {
                type Value = ();
                fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
                    write!(formatter, "a map of component identifiers to packed data")
                }

                fn visit_map<M>(self, mut map: M) -> Result<(), M::Error>
                    where M: _serde::de::MapAccess<'de>
                {
                    #[allow(unused_variables)]
                    while let Some(key) = map.next_key::<&'de str>()? {
                        match key {
                            #(
                                #comp_serialize_name => {
                                    let mut storage = self.0.write_with_id::<#comp_serialize_ty>(#comp_serialize_id);
                                    let packed = map.next_value::<_specs::PackedData<#comp_serialize_ty2>>()?;
                                    let _ = storage.merge(self.1, packed);
                                },
                            )*
                            uncaught_key @ _ => {
                                #(
                                    if let Some(()) = <#subgroup_ty as _specs::SerializeGroup>::deserialize_subgroup(self.0, self.1, uncaught_key, &mut map)? {
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

        fn deserialize_subgroup<'de, M>(world: &mut _specs::World, entities: &[_specs::Entity], key: &'de str, mut map: &mut M) -> Result<Option<()>, M::Error>
            where M: _serde::de::MapAccess<'de>
        {
            #[allow(unused_variables)]
            match key {
                #(
                    #comp_serialize_name => {
                        let mut storage = world.write_with_id::<#comp_serialize_ty>(#comp_serialize_id);
                        let packed = map.next_value::<_specs::PackedData<#comp_serialize_ty2>>()?;
                        let _ = storage.merge(entities, packed);
                        Ok(Some(()))
                    },
                )*
                uncaught_key @ _ => {
                    #(
                        if let Some(()) = #subgroup_ty::deserialize_subgroup(world, entities, uncaught_key, map)? {
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
        impl _specs::ComponentGroup for #name {
            #default
        }
    };

    // Serialization expand
    #[cfg(feature="serialize")]
    let expanded = quote! {
        extern crate serde as _serde;
        #expanded
        #[automatically_derived]
        impl _specs::SerializeGroup for #name {
            #serialize
        }
    };

    // Wrap the expanded code to prevent context conflicts.
    let wrap = quote! {
        #[allow(non_upper_case_globals, unused_attributes, unused_qualifications)]
        #[macro_use]
        const #dummy_const: () = {
            extern crate specs as _specs;
            #deconstructed
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

    // Component id.
    pub id: usize,
}

impl Parameter {
    pub fn default(field: &Field) -> Self {
        Parameter {
            subgroup: false,
            serialize: false,
            ignore: false,
            name: field.ident.clone().unwrap().to_string(),
            id: 0usize,
        }
    }

    pub fn parse(field: &Field) -> Parameter {
        use syn::NestedMetaItem::MetaItem;
        use syn::MetaItem::{NameValue, Word};
            
        let mut params = Parameter::default(field);
        params.name = field.ident.clone().unwrap().to_string();

        for meta_items in field.attrs.iter().filter_map(filter_group) {
            for meta_item in meta_items {
                match meta_item {
                    MetaItem(Word(ref name)) if name == "serialize" => params.serialize = true,
                    MetaItem(Word(ref name)) if name == "subgroup" => params.subgroup = true,
                    MetaItem(Word(ref name)) if name == "ignore" => params.ignore = true,
                    MetaItem(NameValue(ref name, Lit::Str(ref id, _))) if name == "id" => {
                        if let Ok(id) = id.parse::<usize>() {
                            params.id = id;
                        }
                        else {
                            println!("{} is not a valid id", id);
                        }
                    },
                    _ => panic!("Unknown group attribute: {:?}", meta_item),
                }
            }
        }

        params
    }
}

/// Get the attributes related to this derive.
pub fn filter_group(attr: &Attribute) -> Option<Vec<NestedMetaItem>> {
    match attr.value {
        MetaItem::List(ref name, ref items) if name == "group" => Some(items.iter().cloned().collect()),
        _ => None,
    }
}

#[derive(Debug, Clone)]
pub struct Item<'a> {
    pub parameter: Parameter,
    pub field: &'a Field,
}

#[derive(Debug, Clone)]
pub struct ItemList<'a>(Vec<Item<'a>>);

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

    pub fn from_list<'b>(list: Vec<Item<'b>>) -> ItemList<'b> {
        ItemList(list)
    }

    pub fn type_list(&self) -> Ty {
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

    pub fn identifiers(&self) -> Vec<Ident> {
        self.0.iter()
            .map(|item| item.field.ident.clone().unwrap() )
            .collect()
    }

    pub fn names(&self) -> Vec<String> {
        self.0.iter()
            .map(|item| item.field.ident.clone().unwrap().to_string() )
            .collect()
    }

    pub fn types(&self) -> Vec<Ty> {
        self.0.iter()
            .map(|item| item.field.ty.clone() )
            .collect()
    }

    pub fn ids(&self) -> Vec<usize> {
        self.0.iter()
            .map(|item| item.parameter.id )
            .collect()
    }
}
