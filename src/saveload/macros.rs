/// This macro is a convenience wrapper for the most common method of serializing or deserializing components.
///
/// This takes several arguments: a list of components surrounded by brackets `[ Foo, Bar, Baz ]`,
/// the target name of the generated deserialization system data, the
/// target name of the serialization system data, and the target name of the intermediate data struct all your
/// components will be serialized from (essentially a big list of [`<T as IntoSerialize>::Data`]).
///
/// At the site of invocation, it will create a submodule named `saveload_generated` containing the struct names
/// given in the arguments.
///
/// The deserialization derives [`SystemData`] and contains:
///
/// - An opaque field containing the [`WriteStorage`]s to the given components, implementing [`DeserializeComponents`] (named `components`)
/// - A [`WriteStorage`] handle to a generic [`Marker`] (named `markers`)
/// - A [`Write`] handle to a [`MarkerAllocator`] parameterized by said [`Marker`] (named `alloc`)
/// - The [`Entities`] resource (named `entities`)
///
/// The serialization struct also derives [`SystemData`] and contains:
///
/// - An opaque field containing the [`ReadStorage`]s to the given components, implementing [`SerializeComponents`] (named `components`)
/// - A [`ReadStorage`] handle to a generic [`Marker`] (named `markers)
/// - The [`Entities`] resource (named `entities`)
///
/// This is set up so that by making the generated a field in the [`SystemData`] for your [`System`], you automatically have every
/// element necessary to immediately call [`DeserializeComponents::deserialize`] or [`SerializeComponents::serialize`].
///
///
/// Note that this will still play nicely with non-trivial systems, provided you don't ask for any systems you give to this macro
/// again. You can still fetch certain [`Component`]s mutably in your [`SystemData`], for instance, if you wish to derive their values instead of directly
/// serializing them. For example:
///
/// ```rust
/// # #[macro_use]
/// # extern crate specs;
/// # #[macro_use]
/// # extern crate specs_derive;
/// # #[macro_use]
/// # extern crate serde;
/// # #[macro_use]
/// # extern crate shred_derive;
/// # extern crate shred;
/// #
/// # use specs::prelude::*;
/// # #[derive(Copy, Clone, Debug, Serialize, Deserialize, Component)]
/// # #[storage(VecStorage)]
/// # pub struct Foo;
/// #
/// # #[derive(Copy, Clone, Debug, Serialize, Deserialize, Component)]
/// # #[storage(VecStorage)]
/// # pub struct Bar;
/// #
/// # #[derive(Copy, Clone, Debug, Serialize, Deserialize, Component)]
/// # #[storage(VecStorage)]
/// # pub struct Baz;
/// #
/// saveload_components!{[Foo, Bar], Deser, Ser, Data}
///
/// use specs::saveload::{U64MarkerAllocator, U64Marker};
///
/// #[derive(SystemData)]
/// struct NonTrivial<'a> {
///     // We could derive this from any entity with both `Foo` and `Bar` for example
///     baz: WriteStorage<'a, Baz>,
///     de: saveload_generated::Deser<'a, U64Marker, U64MarkerAllocator>,
/// }
/// # // This shadow main has to be here or we get a "macro_use must be declared in the crate root"
/// # // error
/// # fn main() {
/// # }
/// ```
///
/// Will allow us to do things with `Baz` while still giving us a common place to define the components we want
/// to serialize and deserialize (reducing errors because we added one to one collection but not the other).
///
///
///
/// [`<T as IntoSerialize>::Data`]: ./saveload/IntoSerialize.t.html#associatedtype.Data
/// [`DeserializeComponents`]: ./saveload/DeserializeComponents.t.html
/// [`SerializeComponents`]: ./saveload/SerializeComponents.t.html
/// [`DeserializeComponents::deserialize`]: ./saveload/DeserializeComponents.t.html#method.deserialize
/// [`SerializeComponents::serialize`]: ./saveload/SerializeComponents.t.html#method.serialize
/// [`Marker`]: ./saveload/Marker.t.html
/// [`MarkerAllocator`]: ./saveload/MarkerAllocator.t.html
/// [`WriteStorage`]: WriteStorage.t.html
/// [`Write`]: Write.t.html
/// [`Component`]: Component.t.html
/// [`ReadStorage`]: ReadStorage.t.html
/// [`Entities`]: Entities.t.html
/// [`SystemData`]: SystemData.t.html
#[macro_export]
macro_rules! saveload_components {
    ( [ $($name:ident),* ],  $deser_name:ident, $ser_name:ident, $data_name:ident) => {
        mod saveload_generated {
            #![allow(unused_imports)]
            
            use super::*;
            use $crate::saveload::*;
            use $crate::prelude::*;
            use ::serde::{Serialize, Deserialize};

            /// The serialized version of these components
            #[allow(non_snake_case)]
            #[derive(Serialize, Deserialize)]
            #[serde(bound="")]
            #[doc(hidden)]
            pub struct $data_name<MA>
            where
                MA: Marker+::serde::Serialize,
                for<'deser> MA: ::serde::Deserialize<'deser>,
            {
                $(pub $name: Option<<$name as IntoSerialize<MA>>::Data> ),*
            }

            /// The generated deserializer system data
            #[derive(SystemData)]
            #[allow(dead_code)]
            #[doc(hidden)]
            pub struct $deser_name<'a, M: Marker, A: MarkerAllocator<M> > {
                pub markers: WriteStorage<'a, M>,
                pub entities: Entities<'a>,
                pub alloc: Write<'a, A>,
                pub components: self::deser_components::$deser_name<'a>,
            }

            pub use self::deser_components::$deser_name as DeserComponents;

            #[doc(hidden)]
            pub mod deser_components {
                #![allow(unused_imports)]

                use $crate::WriteStorage;
                use $crate::saveload::Marker;
                use super::*;

                #[allow(non_snake_case)]
                #[derive(SystemData)]
                #[doc(hidden)]
                pub struct $deser_name<'a> {
                    $( pub $name: WriteStorage<'a, $name> ),*
                }
            }

            impl<'a, M> DeserializeComponents<$crate::error::NoError, M> for self::deser_components::$deser_name<'a>
            where 
                M: Marker+'a,
            {
                type Data = $data_name<M>;

                fn deserialize_entity<F>(
                    &mut self,
                    entity: Entity,
                    components: Self::Data,
                    mut ids: F,
                ) -> Result<(), $crate::error::NoError>
                where
                    F: FnMut(M) -> Option<Entity> 
                {
                    $(if let Some(comp) = components.$name {
                        let comp: $name = FromDeserialize::from(comp, &mut ids)?;
                        self.$name.insert(entity, comp).unwrap();
                    });*

                    Ok(())
                }
            }

            /// The generated deserializer system data
            #[derive(SystemData)]
            #[allow(dead_code)]
            #[doc(hidden)]
            pub struct $ser_name<'a, M: Marker> {
                pub markers: ReadStorage<'a, M>,
                pub entities: Entities<'a>,
                pub components: self::ser_components::$ser_name<'a>,
            }

            pub use self::ser_components::$ser_name as SerComponents;

            #[doc(hidden)]
            pub mod ser_components {
                #![allow(unused_imports)]

                use $crate::ReadStorage;
                use $crate::saveload::Marker;
                use super::*;

                #[allow(non_snake_case)]
                #[derive(SystemData)]
                #[doc(hidden)]
                pub struct $ser_name<'a> {
                    $( pub $name: ReadStorage<'a, $name> ),*
                }
            }

            impl<'a, M> SerializeComponents<$crate::error::NoError, M> for self::ser_components::$ser_name<'a>
            where 
                M: Marker, 
            {
                type Data = $data_name<M>;

                fn serialize_entity<F>(&self, entity: Entity, mut ids: F) -> Result<Self::Data, $crate::error::NoError>
                where
                    F: FnMut(Entity) -> Option<M>
                {
                    Ok($data_name {
                        $($name: if let Some(comp) = self.$name.get(entity) {
                            Some(IntoSerialize::into(comp, &mut ids).unwrap())
                        } else { 
                            None 
                        }),*   
                    })
                }
            }
        }
    }
}
