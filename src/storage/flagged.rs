use std::marker::PhantomData;

use hibitset::BitSet;

use {Index, Join, UnprotectedStorage};
use world::EntityIndex;

/// Wrapper storage that stores modifications to components in a bitset.
///
/// **Note:** Joining over all components of a `FlaggedStorage`
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
/// use specs::{Component, Entities, FlaggedStorage, Join, System, VecStorage, WriteStorage};
///
/// pub struct Comp(u32);
/// impl Component for Comp {
///     // `FlaggedStorage` acts as a wrapper around another storage.
///     // You can put any store inside of here (e.g. HashMapStorage, VecStorage, etc.)
///     type Storage = FlaggedStorage<Self, VecStorage<Self>>;
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
///         for flagged_comp in ((&comps).open().1).join() {
///             // ...
///         }
///
///         // Clears the tracked storage every frame with this system.
///         (&mut comps).open().1.clear_flags();
///     }
/// }
///# fn main() { }
/// ```
#[derive(Derivative)]
#[derivative(Default(bound = "T: Default"))]
pub struct FlaggedStorage<C, T> {
    mask: BitSet,
    storage: T,
    phantom: PhantomData<C>,
}

impl<C, T: UnprotectedStorage<C>> UnprotectedStorage<C> for FlaggedStorage<C, T> {
    unsafe fn clean<F>(&mut self, has: F)
    where
        F: Fn(Index) -> bool,
    {
        self.mask.clear();
        self.storage.clean(has);
    }

    unsafe fn get(&self, id: Index) -> &C {
        self.storage.get(id)
    }

    unsafe fn get_mut(&mut self, id: Index) -> &mut C {
        // calling `.iter()` on an unconstrained mutable storage will flag everything
        self.mask.add(id);
        self.storage.get_mut(id)
    }

    unsafe fn insert(&mut self, id: Index, comp: C) {
        self.mask.add(id);
        self.storage.insert(id, comp);
    }

    unsafe fn remove(&mut self, id: Index) -> C {
        self.mask.remove(id);
        self.storage.remove(id)
    }
}

impl<C, T: UnprotectedStorage<C>> FlaggedStorage<C, T> {
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

impl<'a, C, T: UnprotectedStorage<C>> Join for &'a FlaggedStorage<C, T> {
    type Type = &'a C;
    type Value = &'a T;
    type Mask = &'a BitSet;

    fn open(self) -> (Self::Mask, Self::Value) {
        (&self.mask, &self.storage)
    }

    unsafe fn get(v: &mut Self::Value, id: Index) -> &'a C {
        v.get(id)
    }
}

impl<'a, C, T: UnprotectedStorage<C>> Join for &'a mut FlaggedStorage<C, T> {
    type Type = &'a mut C;
    type Value = &'a mut T;
    type Mask = &'a BitSet;

    fn open(self) -> (Self::Mask, Self::Value) {
        (&self.mask, &mut self.storage)
    }

    unsafe fn get(v: &mut Self::Value, id: Index) -> &'a mut C {
        // similar issue here as the `Storage<T, A, D>` implementation
        let value: *mut Self::Value = v as *mut Self::Value;
        (*value).get_mut(id)
    }
}
