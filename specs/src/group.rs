
use std::marker::PhantomData;

use prelude::World;

#[cfg(feature="serialize")]
use prelude::Entity;
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
    fn deserialize_group<D: Deserializer>(world: &mut World, entities: &[Entity], deserializer: D) -> Result<(), D::Error>;
    /// Helper method for deserializing the world.
    fn deserialize_subgroup<V>(world: &mut World, entities: &[Entity], key: String, visitor: &mut V) -> Result<Option<()>, V::Error>
        where V: serde::de::MapVisitor;
}

/// Splits a tuple with recursive associated types.
pub trait Split {
    /// The type split off from the tuple.
    type This;
    /// The rest of the tuple aside from the split off associated type.
    type Next: Split;
    /// Is there another split possible.
    fn next() -> bool;
    /// Next split in the iterator.
    fn split_iter(&self, ty: String) -> SplitIter<Self::Next> {
        SplitIter {
            inner: None,
            ty: ty,
        }
    }
}

pub struct SplitConcat<A: Split, B: Split>(PhantomData<(A, B)>);

impl<A: Split, B: Split> Split for SplitConcat {
    type This = A::This;
    type Next = Break<A::Next, (B::This, B::Next)>;
    fn next() -> bool {

    }
}

pub struct Break<A, B>;
impl<A, B> Split for Break {
    type This = A::This;
    type Next = A::Next;
}

pub fn next(s: String) -> String {
    "<".to_owned() + &s + " as Split>::Next"
}

pub fn this(s: String) -> String {
    "<".to_owned() + &s + " as Split>::This"
}

pub struct SplitIter<S>
    where S: Split,
          <S as Split>::Next: Split,
{
    inner: Option<<S as Split>::Next>,
    ty: String,
}

impl<S> Iterator for SplitIter<S>
    where S: Split,
          <S as Split>::Next: Split,
{
    type Item = String;
    fn next(&mut self) -> Option<String> {
        use std::mem;
        if let Some(ref mut inner) = self.inner {
            inner.split_iter(next(self.ty.clone())).next()
        }
        else {
            self.inner = Some(unsafe { mem::uninitialized() });
            if <S as Split>::next() {
                Some(this(self.ty.clone()))
            }
            else {
                None
            }
        }
    }
}

/// Deconstructs the group.
pub trait DeconstructedGroup: ComponentGroup {
    /// All components of this group and all subgroups recursively.
    type All: Split;
    /// Locals of the group.
    type Locals: Split;
    /// Subgroups of the group.
    type Subgroups: Split;
    /// Amount of components of this group and all subgroups recursively.
    fn all() -> usize;
    ///// Parameters associated with the components.
    //fn all_params() -> &'static [Parameter];
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

