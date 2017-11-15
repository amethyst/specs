
use hibitset::BitSet;

use {Index, Join, Metadata};
use world::EntityIndex;
use storage::meta::{Associate, AssociateMut};

/// Wrapper storage that stores modifications to components in a bitset.
///
/// **Note:** Joining over all components of a `Flagged`
/// mutably will flag all components.**
///
/// What you want to instead is to use `check()` or `restrict()` to first
/// get the entities which contain the component,
/// and then conditionally set the component
/// after a call to `get_mut_unchecked()` or `get_mut()`.
///
/// # Examples
///
/// ```rust
/// extern crate specs;
///
/// use specs::{Component, Entities, Flagged, Join, System, DenseVecStorage, WriteStorage};
///
/// #[derive(Metadata)]
/// pub struct CompMetadata {
///    pub flagged: Flagged,
/// }
///
/// pub struct Comp(u32);
/// impl Component for Comp {
///     type Storage = DenseVecStorage<Self>;
///     type Metadata = Flagged;
/// }
///
/// pub struct CompSystem;
/// impl<'a> System<'a> for CompSystem {
///     type SystemData = (Entities<'a>, WriteStorage<'a, Comp>);
///     fn run(&mut self, (entities, mut comps): Self::SystemData) {
///         // Iterates over all components like normal.
///         for comp in (&comps).join() {
///             // ...
///         }
///
///         // **Never do this**
///         // This will flag all components as modified regardless of whether the inner loop
///         // did modify their data.
///         //
///         // Only do this if you have a bunch of other components to filter out the ones you
///         // want to modify.
///         for comp in (&mut comps).join() {
///             // ...
///         }
///
///         // Instead do something like:
///         for (entity, _) in (&*entities, &comps.check()).join() {
///             if true { // check whether this component should be modified.
///                 match comps.get_mut(entity) {
///                     Some(component) => { /* ... */ },
///                     None => { /* ... */ },
///                 }
///             }
///         }
///
///         // Or alternatively:
///         for (entity, (mut entry, mut restrict)) in (&*entities, &mut comps.restrict()).join() {
///             if true { // check whether this component should be modified.
///                  let mut comp = restrict.get_mut_unchecked(&mut entry);
///                  // ...
///             }
///         }
///
///         // To iterate over the flagged/modified components:
///         for flagged_comp in (comps.meta().flagged).join() {
///             // ...
///         }
///
///         // Clears the tracked storage every frame with this system.
///         comps.mut_meta().flagged.clear_flags();
///     }
/// }
///# fn main() { }
/// ```
#[derive(Default)]
pub struct Flagged {
    mask: BitSet,
}

impl<T> Metadata<T> for Flagged {
    fn clean<F>(&mut self, _: &F)
    where
        F: Fn(Index) -> bool
    {
        self.mask.clear();
    }
    fn get_mut(&mut self, id: Index, _: &mut T) {
        self.mask.add(id);
    }
    fn insert(&mut self, id: Index, _: &T) {
        self.mask.add(id);
    }
    fn remove(&mut self, id: Index, _: &T) {
        self.mask.remove(id);
    }
}

impl Flagged {
    /// Whether the component that belongs to the given entity was flagged or not.
    pub fn flagged<E: EntityIndex>(&self, entity: E) -> bool {
        self.mask.contains(entity.index())
    }

    /// All components will be cleared of being flagged.
    pub fn clear_flags(&mut self) {
        self.mask.clear();
    }

    /// Removes the flag for the component of the given entity.
    pub fn unflag<E: EntityIndex>(&mut self, entity: E) {
        self.mask.remove(entity.index());
    }

    /// Flags a single component.
    pub fn flag<E: EntityIndex>(&mut self, entity: E) {
        self.mask.add(entity.index());
    }
}

impl<'a> Join for &'a Flagged {
    type Type = ();
    type Value = ();
    type Mask = &'a BitSet;
    fn open(self) -> (Self::Mask, Self::Value) {
        (&self.mask, ())
    }
    unsafe fn get(v: &mut Self::Value, id: Index) -> Self::Type {
        ()
    }
}

impl<'a, T> Associate<T> for &'a Flagged {
    type Mask = &'a BitSet;
    fn mask(self) -> Self::Mask {
        &self.mask
    }
}

impl<'a, T> AssociateMut<T> for &'a Flagged {
    type Mask = BitSet;
    fn mut_mask(self) -> Self::Mask {
        self.mask.clone()
    }
}
