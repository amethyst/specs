use std::marker::PhantomData;

use shrev::EventChannel;

use storage::{DenseVecStorage, Flag, InsertedFlag, ModifiedFlag, RemovedFlag, Tracked,
              UnprotectedStorage};
use world::Index;

/// Wrapper storage that tracks modifications, insertions, and removals of components
/// through an `EventChannel`.
///
/// **Note:** Joining over all components of a `FlaggedStorage`
/// mutably will flag all components.
///
/// What you want to instead is to use `check()` or `restrict()` to first
/// get the entities which contain the component and then conditionally
/// set the component after a call to `get_mut_unchecked()` or `get_mut()`.
///
/// # Examples
///
/// ```rust
/// extern crate specs;
/// extern crate shrev;
/// extern crate hibitset;
///
/// use specs::prelude::*;
///
/// pub struct Comp(u32);
/// impl Component for Comp {
///     // `FlaggedStorage` acts as a wrapper around another storage.
///     // You can put any store inside of here (e.g. HashMapStorage, VecStorage, etc.)
///     type Storage = FlaggedStorage<Self, VecStorage<Self>>;
/// }
///
/// pub struct CompSystem {
///     // This keeps track of the last modification events the system read.
///     modified_id: ReaderId<ModifiedFlag>,
///     modified: BitSet,
/// }
///
/// impl<'a> System<'a> for CompSystem {
///     type SystemData = (Entities<'a>, WriteStorage<'a, Comp>);
///     fn run(&mut self, (entities, mut comps): Self::SystemData) {
///         // This allows us to use the modification events in a `Join`. Otherwise we
///         // would have to iterate through the events which may not be in order.
///         comps.populate_modified(&mut self.modified_id, &mut self.modified);
///
///         // Iterates over all components like normal.
///         for comp in (&comps).join() {
///             // ...
///         }
///
///         // **Never do this**
///         // This will flag all components as modified regardless of whether the inner loop
///         // actually modified the component.
///         //
///         // Only do this if you have other filters, like some other components to filter
///         // out the ones you want to modify.
///         for comp in (&mut comps).join() {
///             // ...
///         }
///
///         // Instead do something like:
///#        let condition = true;
///         for (entity, (mut entry, mut restrict)) in (&*entities, &mut comps.restrict_mut()).join() {
///             if condition { // check whether this component should be modified.
///                  let mut comp = restrict.get_mut_unchecked(&mut entry);
///                  // ...
///             }
///         }
///
///         // To iterate over the flagged/modified components:
///         for comp in (&comps, &self.modified).join() {
///             // ...
///         }
///     }
/// }
///# fn main() { }
/// ```
pub struct FlaggedStorage<C, T = DenseVecStorage<C>> {
    modified: EventChannel<ModifiedFlag>,
    inserted: EventChannel<InsertedFlag>,
    removed: EventChannel<RemovedFlag>,
    storage: T,
    phantom: PhantomData<C>,
}

impl<C, T> Default for FlaggedStorage<C, T>
where
    T: Default,
{
    fn default() -> Self {
        FlaggedStorage {
            modified: EventChannel::new(),
            inserted: EventChannel::new(),
            removed: EventChannel::new(),
            storage: T::default(),
            phantom: PhantomData,
        }
    }
}

impl<C, T> UnprotectedStorage<C> for FlaggedStorage<C, T>
where
    T: UnprotectedStorage<C>,
{
    unsafe fn clean<F>(&mut self, has: F)
    where
        F: Fn(Index) -> bool,
    {
        self.storage.clean(has);
    }

    unsafe fn get(&self, id: Index) -> &C {
        self.storage.get(id)
    }

    unsafe fn get_mut(&mut self, id: Index) -> &mut C {
        // calling `.iter()` on an unconstrained mutable storage will flag everything
        self.modified.single_write(Flag::Flag(id).into());
        self.storage.get_mut(id)
    }

    unsafe fn insert(&mut self, id: Index, comp: C) {
        self.inserted.single_write(Flag::Flag(id).into());
        self.storage.insert(id, comp);
    }

    unsafe fn remove(&mut self, id: Index) -> C {
        self.removed.single_write(Flag::Flag(id).into());
        self.storage.remove(id)
    }
}

impl<C, T> Tracked for FlaggedStorage<C, T> {
    fn modified(&self) -> &EventChannel<ModifiedFlag> {
        &self.modified
    }
    fn modified_mut(&mut self) -> &mut EventChannel<ModifiedFlag> {
        &mut self.modified
    }
    fn inserted(&self) -> &EventChannel<InsertedFlag> {
        &self.inserted
    }
    fn inserted_mut(&mut self) -> &mut EventChannel<InsertedFlag> {
        &mut self.inserted
    }
    fn removed(&self) -> &EventChannel<RemovedFlag> {
        &self.removed
    }
    fn removed_mut(&mut self) -> &mut EventChannel<RemovedFlag> {
        &mut self.removed
    }
}
