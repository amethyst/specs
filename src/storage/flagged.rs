use std::marker::PhantomData;

use hibitset::BitSetLike;

use storage::TryDefault;
use storage::{ComponentEvent, DenseVecStorage, Tracked, UnprotectedStorage};
use world::{Component, Index};

use shrev::EventChannel;

/// Wrapper storage that tracks modifications, insertions, and removals of components
/// through an `EventChannel`.
///
/// **Note:** Joining over all components of a `FlaggedStorage`
/// mutably will flag all components.
///
/// What you want to instead is to use `restrict_mut()` to first
/// get the entities which contain the component and then conditionally
/// modify the component after a call to `get_mut_unchecked()` or `get_mut()`.
///
/// # Examples
///
/// ```
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
///     reader_id: ReaderId<ComponentEvent>,
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
///         //
///         // It is not okay to only read the events in an interval though as that could
///         // leave behind events which would end up growing the event ring buffer to
///         // extreme sizes.
///         self.modified.clear();
///         self.inserted.clear();
///
///         // Here we can populate the bitsets by iterating over the events.
///         // You can also just iterate over the events without using a bitset, but this
///         // allows us to use them in joins with components.
///         {
///             let events = comps.channel()
///                 .read(&mut self.reader_id)
///                 .collect::<Vec<&ComponentEvent>>();
///             for event in events {
///                 match event {
///                     ComponentEvent::Modified(id) => { self.modified.add(*id); },
///                     ComponentEvent::Inserted(id) => { self.inserted.add(*id); },
///                     _ => { },
///                 };
///             }
///         }
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
///         for (entity, mut comps) in (&entities, &mut comps.restrict_mut()).join() {
///             if condition { // check whether this component should be modified.
///                  let mut comp = comps.get_mut_unchecked();
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
///
/// fn main() {
///     let mut world = World::new();
///     world.register::<Comp>();
///
///     // You will want to register the system `ReaderId`s
///     // before adding/modifying/removing any entities and components.
///     //
///     // Otherwise you won't receive any of the modifications until
///     // you start tracking them.
///     let mut comps = world.write_storage::<Comp>();
///     let comp_system = CompSystem {
///         reader_id: comps.track(),
///         modified: BitSet::new(),
///         inserted: BitSet::new(),
///     };
/// }
/// ```
///
pub struct FlaggedStorage<C, T = DenseVecStorage<C>> {
    channel: EventChannel<ComponentEvent>,
    storage: T,
    phantom: PhantomData<C>,
}

impl<C, T> Default for FlaggedStorage<C, T>
where
    T: TryDefault,
{
    fn default() -> Self {
        FlaggedStorage {
            channel: EventChannel::<ComponentEvent>::default(),
            storage: T::unwrap_default(),
            phantom: PhantomData,
        }
    }
}

impl<C: Component, T: UnprotectedStorage<C>> UnprotectedStorage<C> for FlaggedStorage<C, T> {
    unsafe fn clean<B>(&mut self, has: B)
    where
        B: BitSetLike,
    {
        self.storage.clean(has);
    }

    unsafe fn get(&self, id: Index) -> &C {
        self.storage.get(id)
    }

    unsafe fn get_mut(&mut self, id: Index) -> &mut C {
        // calling `.iter()` on an unconstrained mutable storage will flag everything
        self.channel.single_write(ComponentEvent::Modified(id));
        self.storage.get_mut(id)
    }

    unsafe fn insert(&mut self, id: Index, comp: C) {
        self.channel.single_write(ComponentEvent::Inserted(id));
        self.storage.insert(id, comp);
    }

    unsafe fn remove(&mut self, id: Index) -> C {
        self.channel.single_write(ComponentEvent::Removed(id));
        self.storage.remove(id)
    }
}

impl<C, T> Tracked for FlaggedStorage<C, T> {
    fn channel(&self) -> &EventChannel<ComponentEvent> {
        &self.channel
    }

    fn channel_mut(&mut self) -> &mut EventChannel<ComponentEvent> {
        &mut self.channel
    }
}
