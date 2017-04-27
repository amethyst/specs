
use syn::{Ident, VariantData, Attribute, NestedMetaItem, MetaItem, Body, DeriveInput, Field};
use quote::{Tokens};

pub fn expand_group(input: &DeriveInput) -> Result<Tokens, String> {
    let name = &input.ident;
    let (params, fields) = separate_fields(input);

    let (mut comp_param, mut component) = (Vec::new(), Vec::new());
    let (mut sub_param, mut subgroup) = (Vec::new(), Vec::new());
    for (field, param) in fields.iter().zip(params.iter()) {
        match param.option {
            ParameterType::Component { } => { comp_param.push(param); component.push(field); },
            ParameterType::Subgroup { } => { sub_param.push(param); subgroup.push(field); },
        }
    }
    let dummy_const = Ident::new(format!("_IMPL_COMPONENTGROUP_FOR_{}", name));

    // Duplicate references to the components.
    // `quote` currently doesn't support using the same variable binding twice in a repetition.
    let ref component = component.iter().map(|field| &field.ty).collect::<Vec<_>>();
    let component2 = component;
    let component3 = component;
    let ref subgroup = subgroup.iter().map(|field| &field.ty).collect::<Vec<_>>();

    let default = quote! {
        fn local_components() -> Vec<&'static str> {
            vec![ #( stringify!(#component) ),* ]
        }
        fn components() -> Vec<&'static str> {
            let mut list = <#name as _specs::ComponentGroup>::local_components();
            #(
                for component in <#subgroup as _specs::ComponentGroup>::components() {
                    list.push(component);
                }
            )*
            list
        }
        fn subgroups() -> Vec<&'static str> {
            vec![ #( stringify!(#subgroup), )* ]
        }
    };

    #[cfg(feature="serialize")]
    let serialize = quote! {
        fn serialize_group<S: Serializer>(world: &_specs::World, serializer: S) -> Result<S::Ok, S::Error> {
            use _serde::ser::SerializeMap;
            let mut map = serializer.serialize_map(None)?;
            #(
                map.serialize_key(stringify!(#component))?;
                let storage = world.read::<#component2>().pass();
                map.serialize_value(&storage)?;
            )*

            #(
                <#subgroup as _specs::ComponentGroup>::serialize_subgroup::<S>(world, &mut map)?;
            )*

            map.end()
        }
        fn serialize_subgroup<S: Serializer>(world: &_specs::World, map: &mut S::SerializeMap) -> Result<(), S::Error> {
            use _serde::ser::SerializeMap;
            #(
                map.serialize_key(stringify!(#component))?;
                let storage = world.read::<#component2>().pass();
                map.serialize_value(&storage)?;
            )*

            #(
                <#subgroup as _specs::ComponentGroup>::serialize_subgroup::<S>(world, map)?;
            )*

            Ok(())
        }

        fn deserialize_group<D: Deserializer>(world: &mut _specs::World, entities: &[_specs::Entity], deserializer: D) -> Result<(), D::Error> {
            use std::fmt;
            use _specs::PackedData;

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
                        match &*key {
                            #(
                                stringify!(#component) => {
                                    let mut storage = self.0.write::<#component2>().pass();
                                    let packed = visitor.visit_value::<PackedData<#component3>>()?;
                                    storage.merge(self.1, packed);
                                },
                            )*
                            key @ _ => {
                                #(
                                    if let Some(()) = <#subgroup as _specs::ComponentGroup>::deserialize_subgroup(self.0, self.1, key.to_owned(), &mut visitor)? {
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
            match &*key {
                #(
                    stringify!(#component) => {
                        let mut storage = world.write::<#component2>().pass();
                        let packed = visitor.visit_value::<_specs::PackedData<#component3>>()?;
                        storage.merge(entities, packed);
                        Ok(Some(()))
                    },
                )*
                key @ _ => {
                    #(
                        if let Some(()) = <#subgroup as _specs::ComponentGroup>::deserialize_subgroup(world, entities, key.to_owned(), visitor)? {
                            return Ok(Some(()));
                        }
                    )*
                    Ok(None)
                },
            }
        }
    };

    #[cfg(not(feature="serialize"))]
    let expanded = quote! {
        impl specs::ComponentGroup for #name {
            #default
        }
    };

    #[cfg(feature="serialize")]
    let expanded = quote! {
        extern crate serde as _serde;
        impl specs::ComponentGroup for #name {
            #default
            #serialize
        }
    };

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
        use syn::MetaItem::Word;

        let mut subgroup = false;
        let mut serialize = true;            

        for meta_items in input.iter().filter_map(filter_group) {
            for meta_item in meta_items {
                match meta_item {
                    MetaItem(Word(ref name)) if name == "no_serialize" => serialize = false,
                    MetaItem(Word(ref name)) if name == "subgroup" => subgroup = true,
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
        Body::Enum(_) => panic!("Enum cannot be a component group"),
        Body::Struct(ref data) => match data {
            &VariantData::Struct(ref fields) => fields,
            _ => panic!("Struct must have named fields"),
        },
    };

    let parameters = fields.iter().map(|field| Parameter::parse(&field.attrs) ).collect();
    (parameters, fields)
}

