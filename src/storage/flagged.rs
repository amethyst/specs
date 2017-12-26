use std::marker::PhantomData;

use shrev::EventChannel;

use storage::{DenseVecStorage, InsertedFlag, ModifiedFlag, RemovedFlag, Tracked,
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
///
/// use specs::prelude::*;
///
/// pub struct Comp(u32);
/// impl Component for Comp {
///     // `FlaggedStorage` acts as a wrapper around another storage.
///     // You can put any store inside of here (e.g. HashMapStorage, VecStorage, etc.)
///     //
///     // It also works as `FlaggedStorage<Self>` and defaults to `DenseVecStorage<Self>`
///     // for the inner storage.
///     type Storage = FlaggedStorage<Self, VecStorage<Self>>;
/// }
///
/// pub struct CompSystem {
///     // These keep track of where you left off in the event channel.
///     modified_id: ReaderId<ModifiedFlag>,
///     inserted_id: ReaderId<InsertedFlag>,
///
///     // The bitsets you want to populate with modification/insertion events.
///     modified: BitSet,
///     inserted: BitSet,
/// }
///
/// impl<'a> System<'a> for CompSystem {
///     type SystemData = (Entities<'a>, WriteStorage<'a, Comp>);
///     fn run(&mut self, (entities, mut comps): Self::SystemData) {
///         // We want to clear the bitset first so we don't have left over events
///         // from the last frame.
///         //
///         // However, if you want to accumulate changes over a couple frames then you
///         // can only clear it when necessary. (This might be useful if you have some
///         // sort of "tick" system in your game and only want to do operations every
///         // 1/4th of a second or something)
///         self.modified.clear();
///         self.inserted.clear();
///
///         // This allows us to use the modification events in a `Join`. Otherwise we
///         // would have to iterate through the events which may not be in order.
///         //
///         // This does not populate the bitset with inserted components, only pre-existing
///         // components that were changed by a `get_mut` call to the storage.
///         comps.populate_modified(&mut self.modified_id, &mut self.modified);
///
///         // This will only include inserted components from last read, note that this
///         // will not include `insert` calls if there already was a pre-existing component.
///         comps.populate_inserted(&mut self.inserted_id, &mut self.inserted);
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
///         for (entity, (entry, mut restrict)) in (&*entities, &mut comps.restrict_mut()).join() {
///             if condition { // check whether this component should be modified.
///                  let mut comp = restrict.get_mut_unchecked(&entry);
///                  // ...
///             }
///         }
///
///         // To iterate over the modified components:
///         for comp in (&comps, &self.modified).join() {
///             // ...
///         }
///
///         // To iterate over all inserted/modified components;
///         for comp in (&comps, &self.modified & &self.inserted).join() {
///             // ...
///         }
///     }
/// }
/// fn main() {
///     let mut world = World::new();
///     world.register::<Comp>();
///
///     // You will want to register the system `ReaderId`s
///     // before adding/modifying/removing any entities and components.
///     //
///     // Otherwise you won't receive any of the modifications until
///     // you start tracking them.
///     let mut comps = world.write::<Comp>();
///     let comp_system = CompSystem {
///         modified_id: comps.track_modified(),
///         inserted_id: comps.track_inserted(),
///         modified: BitSet::new(),
///         inserted: BitSet::new(),
///     };
/// }
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
        self.modified.single_write(id.into());
        self.storage.get_mut(id)
    }

    unsafe fn insert(&mut self, id: Index, comp: C) {
        self.inserted.single_write(id.into());
        self.storage.insert(id, comp);
    }

    unsafe fn remove(&mut self, id: Index) -> C {
        self.removed.single_write(id.into());
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
