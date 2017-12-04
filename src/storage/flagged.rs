
use std::marker::PhantomData;

use shrev::EventChannel;

use {Flag, Index, Tracked, UnprotectedStorage};

const MODIFY_CAPACITY: usize = 5000;
const INSERT_CAPACITY: usize = 3000;
const REMOVE_CAPACITY: usize = 3000;

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
pub struct FlaggedStorage<C, T> {
    modified: EventChannel<Flag>,
    inserted: EventChannel<Flag>,
    removed: EventChannel<Flag>,
    storage: T,
    phantom: PhantomData<C>,
}

impl<C, T> Default for FlaggedStorage<C, T>
where
    T: Default
{
    fn default() -> Self {
        FlaggedStorage {
            modified: EventChannel::with_capacity(MODIFY_CAPACITY),
            inserted: EventChannel::with_capacity(INSERT_CAPACITY),
            removed: EventChannel::with_capacity(REMOVE_CAPACITY),
            storage: T::default(),
            phantom: PhantomData,
        }
    }
}

impl<C, T: UnprotectedStorage<C>> UnprotectedStorage<C> for FlaggedStorage<C, T> {
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
        self.modified.single_write(Flag::Flag(id));
        self.storage.get_mut(id)
    }

    unsafe fn insert(&mut self, id: Index, comp: C) {
        self.inserted.single_write(Flag::Flag(id));
        self.storage.insert(id, comp);
    }

    unsafe fn remove(&mut self, id: Index) -> C {
        self.removed.single_write(Flag::Flag(id));
        self.storage.remove(id)
    }
}

impl<C, T> Tracked for FlaggedStorage<C, T> {
    fn modified(&self) -> &EventChannel<Flag> {
        &self.modified
    }
    fn modified_mut(&mut self) -> &mut EventChannel<Flag> {
        &mut self.modified
    }
    fn inserted(&self) -> &EventChannel<Flag> {
        &self.inserted
    }
    fn inserted_mut(&mut self) -> &mut EventChannel<Flag> {
        &mut self.inserted
    }
    fn removed(&self) -> &EventChannel<Flag> {
        &self.removed
    }
    fn removed_mut(&mut self) -> &mut EventChannel<Flag> {
        &mut self.removed
    }
}

