
use World;

#[cfg(feature="serialize")]
use Entity;
#[cfg(feature="serialize")]
use serde::{self, Serializer, Deserializer};

/// Group of components. Can be subgrouped into other component groups.
pub trait ComponentGroup {
    /// Components defined in this group, not a subgroup.
    fn local_components() -> Vec<&'static str>;
    /// Components defined in this group along with subgroups.
    fn components() -> Vec<&'static str>;
    /// Subgroups included in this group.
    fn subgroups() -> Vec<&'static str>;
    /// Registers the components into the world.
    fn register(&mut World);
}

/// Group of serializable components.
#[cfg(feature="serialize")]
pub trait SerializeGroup: ComponentGroup {
    /// Serializes the group of components from the world.
    fn serialize_group<S: Serializer>(world: &World, serializer: S) -> Result<S::Ok, S::Error>;
    /// Helper method for serializing the world.
    fn serialize_subgroup<S: Serializer>(world: &World, map: &mut S::SerializeMap) -> Result<(), S::Error>;
    /// Deserializes the group of components into the world.
    fn deserialize_group<'de, D: Deserializer<'de>>(world: &mut World, entities: &[Entity], deserializer: D) -> Result<(), D::Error>;
    /// Helper method for deserializing the world.
    fn deserialize_subgroup<'de, M>(world: &mut World, entities: &[Entity], key: &'de str, map: &mut M) -> Result<Option<()>, M::Error>
        where M: serde::de::MapAccess<'de>;
}

/// Splits a tuple with recursive associated types.
pub trait Split {
    /// The type split off from the tuple.
    type This;
    /// The rest of the tuple aside from the split off associated type.
    type Next: Split;
    /// Is there another split possible.
    fn next() -> bool;
}

/// Deconstructs the group.
pub trait DeconstructedGroup: ComponentGroup {
    /// Locals of the group.
    type Locals: Split;
    /// Subgroups of the group.
    type Subgroups: Split;
    /// Amount of local components there are in this group.
    fn locals() -> usize;
    /// Amount of subgroups there are in this group.
    fn subgroups() -> usize;
}

impl<A, B: Split> Split for (A, B) {
    type This = A;
    type Next = B;
    fn next() -> bool { true }
}

impl<A> Split for (A,) {
    type This = A;
    type Next = ();
    fn next() -> bool { false }
}

impl Split for () {
    type This = ();
    type Next = ();
    fn next() -> bool { false }
}

#[macro_export]
macro_rules! call {
    // Top level calls
    ( component: $group:ty => 
        fn $method:ident
        [ $( $before:ty ),* ] in [ $( $after:ty ),* ]
        ( $( $args:expr ),* )
    ) => {
        call!(
            @Iter $group: Locals =>
            fn $method
            [ $( $before ),* ] in [ $( $after ),* ]
            ( $( $args ),* )
        );
    };
    ( subgroup: $group:ty => 
        fn $method:ident
        [ $( $before:ty ),* ] in [ $( $after:ty ),* ]
        ( $( $args:expr ),* )
    ) => {
        call!(
            @Iter $group: Subgroups =>
            fn $method
            [ $( $before ),* ] in [ $( $after ),* ]
            ( $( $args ),* )
        );
    };

    // Helper methods

    // Break macro recursion.
    (@Iter [ $( $group:tt )* ] [ ] =>
        fn $method:ident
        [ $( $before:ty ),* ] in [ $( $after:ty ),* ]
        ( $( $args:expr ),* )
    ) => { };

    // Iterate over components through associated type recursion.
    (@Iter [ $( $group:tt )* ] [ A $( $token:tt )* ] =>
        fn $method:ident
        [ $( $before:ty ),* ] in [ $( $after:ty ),* ]
        ( $( $args:expr ),* )
    ) => {
        $method::<$( $before, )* <$( $group )* as Split>::This $( , $after )*>( $( $args ),* );

        if <$( $group )* as Split>::next() {
            call!(
                @Iter [ <$( $group )* as Split>::Next ] [ $( $token )* ] =>
                fn $method
                [ $( $before ),* ] in [ $( $after ),* ]
                ( $( $args ),* )
            );
        }
    };

    // Entrance to iteration.
    (@Iter $group:ty: $at:ident =>
        fn $method:ident
        [ $( $before:ty ),* ] in [ $( $after:ty ),* ]
        ( $( $args:expr ),* )
    ) => {
        call!(
            @Iter [ <$group as DeconstructedGroup>::$at ] [
                // Requires tokens for recursion breaking.
                // Each row has 32 tokens, totalling to 128 currently.
                A A A A A A A A A A A A A A A A A A A A A A A A A A A A A A A A
                A A A A A A A A A A A A A A A A A A A A A A A A A A A A A A A A
                A A A A A A A A A A A A A A A A A A A A A A A A A A A A A A A A
                A A A A A A A A A A A A A A A A A A A A A A A A A A A A A A A A
            ] =>
            fn $method
            [ $( $before ),* ] in [ $( $after ),* ]
            ( $( $args ),* )
        );
    };
}
