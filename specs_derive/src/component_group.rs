
use syn::{Ident, VariantData, Attribute, NestedMetaItem, MetaItem, Body, DeriveInput, Field};
use quote::{ToTokens, Tokens};

/// Expands the `ComponentGroup` derive implementation.
#[allow(unused_variables)]
pub fn expand_group(input: &DeriveInput) -> Result<Tokens, String> {
    let name = &input.ident;
    let items = get_items(input);

    let dummy_const = Ident::new(format!("_IMPL_COMPONENTGROUP_FOR_{}", name));

    // Duplicate references to the components.
    // `quote` currently doesn't support using the same variable binding twice in a repetition.

    let items = items.iter().filter(|item| !item.parameter.ignore).collect::<Vec<_>>();

    // Component fields
    let ref component = items.iter().filter(|item| item.parameter.option.is_component()).collect::<Vec<_>>();
    let component2 = component;
    let component3 = component;

    // Serializable components
    let ref component_serialize = component.iter().filter(|item| item.parameter.serialize).collect::<Vec<_>>();
    let component_serialize2 = component_serialize;

    // Serializable component names
    let ref component_serialize_field = component_serialize.iter().map(|item| item.field.ident.as_ref().unwrap() ).collect::<Vec<_>>();

    // Subgroup fields
    let ref subgroup = items.iter().filter(|item| item.parameter.option.is_subgroup()).collect::<Vec<_>>();

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
                        match &*key {
                            #(
                                stringify!(#component_serialize_field) => {
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
            match &*key {
                #(
                    stringify!(#component_serialize_field) => {
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
        const #dummy_const: () = {
            extern crate specs as _specs;
            #expanded
        };
    };

    Ok(wrap)
}

#[derive(PartialEq, Debug)]
pub enum ParameterType {
    // Parameters relating to subgroups only
    Subgroup {
        
    },
    // Parameters relating to components only
    Component {

    },
}

impl ParameterType {
    fn is_subgroup(&self) -> bool {
        match self {
            &ParameterType::Subgroup { } => true,
            &ParameterType::Component { } => false,
        }
    }
    fn is_component(&self) -> bool {
        match self {
            &ParameterType::Subgroup { } => false,
            &ParameterType::Component { } => true,
        }
    }
}

#[derive(Debug)]
pub struct Parameter {
    // Type of parameter
    pub option: ParameterType,

    // Serialize this field
    pub serialize: bool,

    // Ignore this field completely
    pub ignore: bool,
}

impl Parameter {
    pub fn parse(input: &Vec<Attribute>) -> Parameter {
        use syn::NestedMetaItem::MetaItem;
        use syn::MetaItem::Word;

        let mut subgroup = false;
        let mut serialize = false;
        let mut ignore = false;

        for meta_items in input.iter().filter_map(filter_group) {
            for meta_item in meta_items {
                match meta_item {
                    MetaItem(Word(ref name)) if name == "serialize" => serialize = true,
                    MetaItem(Word(ref name)) if name == "subgroup" => subgroup = true,
                    MetaItem(Word(ref name)) if name == "ignore" => ignore = true,
                    _ => println!("Unused attribute: {:?}", meta_item),
                }
            }
        }

        let parameter_type = if subgroup {
            ParameterType::Subgroup { }
        }
        else {
            ParameterType::Component { }
        };

        Parameter {
            option: parameter_type,
            serialize: serialize,
            ignore: ignore,
        }
    }
}

fn filter_group(attr: &Attribute) -> Option<Vec<NestedMetaItem>> {
    match attr.value {
        MetaItem::List(ref name, ref items) if name == "group" => Some(items.iter().cloned().collect()),
        _ => None,
    }
}

fn get_items(input: &DeriveInput) -> Vec<Item> {
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
                parameter: Parameter::parse(&field.attrs),
                field: field,
            }
        })
        .collect::<Vec<Item>>()
}

struct Item<'a> {
    pub parameter: Parameter,
    pub field: &'a Field,
}

impl<'a> ToTokens for Item<'a> {
    fn to_tokens(&self, tokens: &mut Tokens) {
        self.field.ty.to_tokens(tokens);
    }
}

