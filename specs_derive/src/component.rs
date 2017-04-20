
use serde::{self, Serialize, Serializer, Deserialize, Deserializer};
use serde::de::DeserializeSeed;
use specs::{self, Gate, Join};

use syn::{self, VariantData, Attribute, NestedMetaItem, MetaItem, Body, DeriveInput, Field};
use quote::{Tokens};

use std::marker::PhantomData;

pub trait ComponentGroup {
    /// Components defined in this group, not a subgroup.
    fn local_components() -> Vec<&'static str>;
    /// Components defined in this group along with subgroups.
    fn components() -> Vec<&'static str>;
    /// Subgroups included in this group.
    fn subgroups() -> Vec<&'static str>;
    /// Serializes the group of components from the world.
    fn serialize_group<S: Serializer>(world: &specs::World, serializer: S) -> Result<S::Ok, S::Error>;
    /// Helper method for serializing the world.
    fn serialize_subgroup<S: Serializer>(world: &specs::World, map: &mut S::SerializeMap) -> Result<(), S::Error>;
    /// Deserializes the group of components into the world.
    fn deserialize_group<D: Deserializer>(world: &mut specs::World, entities: &[specs::Entity], deserializer: D) -> Result<(), D::Error>;
    /// Helper method for deserializing the world.
    fn deserialize_subgroup<V>(world: &mut specs::World, entities: &[specs::Entity], key: String, visitor: &mut V) -> Result<Option<()>, V::Error>
        where V: serde::de::MapVisitor;
}

pub fn expand_group(input: &DeriveInput) -> Result<Tokens, String> {
    let name = &input.ident;
    let (params, fields) = separate_fields(input);

    println!("name: {:?}", name);
    println!("params: {:?}", params);
    println!("fields: {:?}", fields);

    //let components = fields.zip(params).filter(|f, p| p.subgroup).collect();

    /*
    let expanded = quote! {
        impl ComponentGroup for #name {
            fn local_components() -> Vec<&'static str> {
                vec![ #( stringify!(#component), )* ]
            }
            fn components() -> Vec<&'static str> {
                let mut list = <$name as Group>::local_components();
                #(
                    for component in <#subgroup as Group>::components() {
                        list.push(component);
                    }
                )*
                list
            }
            fn subgroups() -> Vec<&'static str> {
                vec![ #( stringify!(#subgroup), )* ]
            }
            fn serialize_group<S: Serializer>(world: &specs::World, serializer: S) -> Result<S::Ok, S::Error> {
                use serde::ser::SerializeMap;
                let mut map = serializer.serialize_map(None)?;
                #(
                    map.serialize_key(stringify!(#component))?;
                    let storage = world.read::<#component>().pass();
                    map.serialize_value(&storage)?;
                )*

                #(
                    <#subgroup as Group>::serialize_subgroup::<S>(world, &mut map)?;
                )*

                map.end()
            }

            fn serialize_subgroup<S: Serializer>(world: &specs::World, map: &mut S::SerializeMap) -> Result<(), S::Error> {
                use serde::ser::SerializeMap;
                #(
                    map.serialize_key(stringify!(#component))?;
                    let storage = world.read::<#component>().pass();
                    map.serialize_value(&storage)?;
                )*

                #(
                    <#subgroup as Group>::serialize_subgroup::<S>(world, map)?;
                )*

                Ok(())
            }

            fn deserialize_group<D: Deserializer>(world: &mut specs::World, entities: &[specs::Entity], deserializer: D) -> Result<(), D::Error> {
                use std::fmt;
                use specs::PackedData;

                struct ComponentVisitor<'a>(&'a mut specs::World, &'a [specs::Entity]);
                impl<'a> serde::de::Visitor for ComponentVisitor<'a> {
                    type Value = ();
                    fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
                        write!(formatter, "a map of component identifiers to packed data")
                    }

                    fn visit_map<V>(self, mut visitor: V) -> Result<(), V::Error>
                        where V: serde::de::MapVisitor
                    {
                        while let Some(key) = visitor.visit_key::<String>()? {
                            match &*key {
                                #(
                                    stringify!(#component) => {
                                        let mut storage = self.0.write::<#component>().pass();
                                        let packed = visitor.visit_value::<PackedData<#component>>()?;
                                        storage.merge(self.1, packed);
                                    },
                                )*
                                key @ _ => {
                                    #(
                                        if let Some(()) = <#subgroup as Group>::deserialize_subgroup(self.0, self.1, key.to_owned(), &mut visitor)? {
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

            fn deserialize_subgroup<V>(world: &mut specs::World, entities: &[specs::Entity], key: String, mut visitor: &mut V) -> Result<Option<()>, V::Error>
                where V: serde::de::MapVisitor
            {
                match &*key {
                    #(
                        stringify!(#component) => {
                            let mut storage = world.write::<#component>().pass();
                            let packed = visitor.visit_value::<specs::PackedData<#component>>()?;
                            storage.merge(entities, packed);
                            Ok(Some(()))
                        },
                    )*
                    key @ _ => {
                        #(
                            if let Some(()) = <#subgroup as Group>::deserialize_subgroup(world, entities, key.to_owned(), visitor)? {
                                return Ok(Some(()));
                            }
                        )*
                        Ok(None)
                    },
                }
            }
        }
    };
    */

    let expanded = quote! {

    };

    Ok(expanded)
}

#[derive(Debug)]
enum ParameterType {
    // Parameters relating to subgroups only
    Subgroup {
        
    },
    // Parameters relating to components only
    Component {

    },
}

#[derive(Debug)]
struct Parameter {
    // Type of parameter
    pub option: ParameterType,

    // Serialize this field
    pub serialize: bool,
}

impl Parameter {
    pub fn parse(input: &Vec<Attribute>) -> Parameter {
        use syn::NestedMetaItem::MetaItem;
        use syn::MetaItem::{Word, NameValue};

        let mut component = ParameterType::Component { };
        let mut subgroup = ParameterType::Subgroup { };
        let mut is_subgroup = false;
        let mut serialize = true;            

        for meta_items in input.iter().filter_map(filter_group) {
            for meta_item in meta_items {
                match meta_item {
                    MetaItem(Word(ref name)) if name == "no_serialize" => serialize = false,
                    MetaItem(Word(ref name)) if name == "subgroup" => {
                    },
                    _ => println!("Unused attribute: {:?}", meta_item),
                }
            }
        }

        Parameter {
            option: parameter_type,
            serialize: serialize,
        }
    }
}

pub fn filter_group(attr: &Attribute) -> Option<Vec<NestedMetaItem>> {
    match attr.value {
        MetaItem::List(ref name, ref items) if name == "group" => Some(items.iter().cloned().collect()),
        _ => None,
    }
}

fn separate_fields(input: &DeriveInput) -> (Vec<Parameter>, &Vec<Field>) {
    let fields = match input.body {
        Body::Enum(ref variants) => panic!("Enum cannot be a component group"),
        Body::Struct(ref data) => match data {
            &VariantData::Struct(ref fields) => fields,
            _ => panic!("Struct must have named fields"),
        },
    };

    let parameters = fields.iter().map(|field| Parameter::parse(&field.attrs) ).collect();
    (parameters, fields)
}

