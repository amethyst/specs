use std::{
    marker::PhantomData,
    ops::{Deref, DerefMut},
    sync::Mutex,
};

use hibitset::BitSetLike;

#[cfg(feature = "parallel")]
use crate::join::ParJoin;
use crate::{
    join::Join,
    storage::{
        ComponentEvent, DenseVecStorage, DistinctStorage, MaskedStorage, Storage, Tracked,
        TryDefault, UnprotectedStorage,
    },
    world::{Component, Entity, Index},
};

use hibitset::BitSet;
use shrev::EventChannel;

/// Wrapper storage that tracks modifications, insertions, and removals of
/// components through an `EventChannel`, in a similar manner to `FlaggedStorage`.
///
/// Unlike `FlaggedStorage`, this storage uses a wrapper type for mutable
/// accesses that only emits modification events when the component is explicitly
/// accessed mutably which. This means that simply performing a mutable join will
/// not, by itself, trigger a modification event.
///
/// To use this storage in a mutable join it should first be split into an [SplitFlaggedStorage]
/// and an [EventPool] using [Storage::split]. This is because the deferred mutable
/// access through the wrapper type requires passing in the channel separately. This strategy is
/// neccessary for soundness because [JoinIter] is not a streaming iterator so if we were to place
/// a mutable reference to the event channel inside of the mutable component wrapper this will
/// cause aliasing of mutable references. In addition, this strategy enables us to safely perform
/// parallel mutable joins with components in this storage.
///
/// Note: no effort is made to ensure a particular ordering of the modification
/// events that occur withing the scope of a single `split`.
pub struct UnsplitFlaggedStorage<C, T = DenseVecStorage<C>> {
    channel: EventChannel<ComponentEvent>,
    storage: T,
    #[cfg(feature = "storage-event-control")]
    event_emission: bool,
    phantom: PhantomData<C>,
}

/// Pool of component modification events.
///
/// In the single threaded `.join()` case this can be used directly for tracked
/// access of mutable components through [SplitFlaggedAccessMut]. For the parrallel
/// `.par_join()` a collector for each rayon job can be created using [Self::collector].
///
/// If you leak this (e.g. with `mem::forget`) the events will never be sent.
pub struct EventPool<'a, C> {
    channel: &'a mut EventChannel<ComponentEvent>,
    pool: Mutex<Vec<Vec<Index>>>,
    phantom: PhantomData<C>,
}

/// If you leak this (e.g. with `mem::forget`) the events will never be sent.
pub struct EventCollector<'a, C> {
    pool: &'a Mutex<Vec<Vec<Index>>>,
    pending_events: Vec<Index>,
    phantom: PhantomData<C>,
}

impl<C> EventPool<'_, C> {
    /// Create a new `EventCollector` for collecting events
    /// This method can be called in the init closure of rayon methods like
    /// `for_each_init` in order to produce a collector as needed for each rayon
    /// job and collect component modification events in parallel.
    pub fn collector(&self) -> EventCollector<'_, C> {
        EventCollector {
            pool: &self.pool,
            pending_events: Vec::new(),
            phantom: PhantomData,
        }
    }
}

impl<'a, C> Drop for EventPool<'a, C> {
    fn drop(&mut self) {
        let event_pool = core::mem::take(self.pool.get_mut().unwrap_or_else(|e| e.into_inner()));
        // Send events to through channel
        event_pool.into_iter().for_each(|events| {
            self.channel
                .iter_write(events.into_iter().map(|id| ComponentEvent::Modified(id)));
        })
    }
}

impl<'a, C> Drop for EventCollector<'a, C> {
    fn drop(&mut self) {
        let events = core::mem::take(&mut self.pending_events);
        // Ignore poison
        let mut guard = self.pool.lock().unwrap_or_else(|e| e.into_inner());
        // Add locally collected events to the pool
        guard.push(events);
    }
}

/// Abstracts over [EventPool] and [EventCollector] so they can both be used for
/// tracked access of components to reduce complexity of the single threaded `.join()`
/// case.
///
/// This trait cannot be implemented for types outside this crate. Actual methods
/// are in a private super trait.
pub trait EventSink<C>: private::EventSink<C> {} // TODO: EventCollector seems like a better name but that overlaps with the type named that :/

impl<T, C> EventSink<C> for T where T: private::EventSink<C> {}

// https://rust-lang.github.io/api-guidelines/future-proofing.html#sealed-traits-protect-against-downstream-implementations-c-sealed
mod private {
    pub trait EventSink<C> {
        fn vec_mut(&mut self) -> &mut Vec<super::Index>;
    }
}

impl<C> private::EventSink<C> for EventPool<'_, C> {
    fn vec_mut(&mut self) -> &mut Vec<Index> {
        // Reach into mutex ignoring poison
        let event_pool = self.pool.get_mut().unwrap_or_else(|e| e.into_inner());

        // If the vec of vecs is empty add one
        if event_pool.is_empty() {
            event_pool.push(Vec::new());
        }

        // Return mutable reference to the first vec
        &mut event_pool[0]
    }
}

impl<C> private::EventSink<C> for EventCollector<'_, C> {
    fn vec_mut(&mut self) -> &mut Vec<Index> {
        &mut self.pending_events
    }
}

pub struct SplitFlaggedStorage<'a, C, T> {
    // Deconstruction of Storage so we can impl Join/ParJoin
    //
    // struct MaskedStorage<T: Component>
    //     mask: BitSet,
    //     inner: T::Storage,
    //
    // struct Storage<'e, T, D>
    //     data: D, // MaskedStorage<T>
    //     entities: Fetch<'e, EntitiesRes>,
    //     phantom: PhantomData<T>,
    mask: &'a BitSet,
    storage: &'a mut T,

    #[cfg(feature = "storage-event-control")]
    event_emission: bool,
    // Invariant lifetime brand
    phantom: PhantomData<C>,
}

impl<'e, C, D, T> Storage<'e, C, D>
where
    C: Component<Storage = UnsplitFlaggedStorage<C, T>>,
    D: DerefMut<Target = MaskedStorage<C>>,
    T: UnprotectedStorage<C>,
{
    /// Temporarily divide into a [SplitFlaggedStorage] which borrows the underlying
    /// storage and an [EventPool] which borrows the underlying event channel and
    /// is used to collect modification events. This allows deferred mutable access of components
    /// in mutable.
    pub fn split(&mut self) -> (SplitFlaggedStorage<'_, C, T>, EventPool<'_, C>) {
        let masked_storage = self.data.deref_mut();
        let split_storage = SplitFlaggedStorage {
            mask: &masked_storage.mask,
            storage: &mut masked_storage.inner.storage,

            // TODO: add method on SplitFlaggedStorage to toggle this?
            #[cfg(feature = "storage-event-control")]
            event_emission: self.event_emission,
            phantom: PhantomData,
        };

        let pool = EventPool {
            channel: &mut masked_storage.inner.channel,
            pool: Mutex::new(Vec::new()),
            phantom: PhantomData,
        };

        (split_storage, pool)
    }
}

impl<C: Component, T: UnprotectedStorage<C>> UnsplitFlaggedStorage<C, T> {
    /// Access the data associated with an `Index` for mutation. Internally emits
    /// a component modification event if event emission is not disabled for this
    /// storage.
    ///
    /// # Safety:
    ///
    /// Same requirements as [UnprotectedStorage::get_mut]
    pub unsafe fn get_mut_tracked(&mut self, id: Index) -> T::AccessMut<'_> {
        if self.emit_event() {
            self.channel.single_write(ComponentEvent::Modified(id));
        }
        self.storage.get_access_mut(id)
    }
}

impl<'e, T, D, I> Storage<'e, T, D>
where
    T: Component<Storage = UnsplitFlaggedStorage<T, I>>,
    D: DerefMut<Target = MaskedStorage<T>>,
    I: UnprotectedStorage<T>,
{
    /// Tries to mutate the data associated with an `Entity`. Internally emits
    /// a component modification event if event emission is not disabled for this
    /// storage. Conveinence method for [UnsplitFlaggedStorage] which normally requires
    /// .... TODO: describe this when details crystallize.
    pub fn get_mut_tracked(&mut self, e: Entity) -> Option<I::AccessMut<'_>> {
        if self.data.mask.contains(e.id()) && self.entities.is_alive(e) {
            // SAFETY: We checked the mask, so all invariants are met.
            Some(unsafe { self.data.inner.get_mut_tracked(e.id()) })
        } else {
            None
        }
    }
}

impl<C, T> UnsplitFlaggedStorage<C, T> {
    #[cfg(feature = "storage-event-control")]
    fn emit_event(&self) -> bool {
        self.event_emission
    }

    #[cfg(not(feature = "storage-event-control"))]
    fn emit_event(&self) -> bool {
        true
    }
}

impl<C, T> Default for UnsplitFlaggedStorage<C, T>
where
    T: TryDefault,
{
    fn default() -> Self {
        Self {
            channel: EventChannel::<ComponentEvent>::default(),
            storage: T::unwrap_default(),
            #[cfg(feature = "storage-event-control")]
            event_emission: true,
            phantom: PhantomData,
        }
    }
}

impl<C: Component, T: UnprotectedStorage<C>> UnprotectedStorage<C> for UnsplitFlaggedStorage<C, T> {
    // UnsplitFlaggedStorage is meant to be split into SplitFlaggedStorage and a EventCollector
    // before mutable joins so the AccessMut type for the unsplit storage that is
    // the unit type
    // TODO: Ideally Self wouldn't be able to be mutably joined over at all
    #[rustfmt::skip]
    type AccessMut<'a> where T: 'a = ();

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
        if self.emit_event() {
            self.channel.single_write(ComponentEvent::Modified(id));
        }

        self.storage.get_mut(id)
    }

    unsafe fn get_access_mut(&mut self, _id: Index) -> Self::AccessMut<'_> {}

    unsafe fn insert(&mut self, id: Index, comp: C) {
        if self.emit_event() {
            self.channel.single_write(ComponentEvent::Inserted(id));
        }
        self.storage.insert(id, comp);
    }

    unsafe fn remove(&mut self, id: Index) -> C {
        if self.emit_event() {
            self.channel.single_write(ComponentEvent::Removed(id));
        }
        self.storage.remove(id)
    }
}

impl<C, T> Tracked for UnsplitFlaggedStorage<C, T> {
    fn channel(&self) -> &EventChannel<ComponentEvent> {
        &self.channel
    }

    fn channel_mut(&mut self) -> &mut EventChannel<ComponentEvent> {
        &mut self.channel
    }

    #[cfg(feature = "storage-event-control")]
    fn set_event_emission(&mut self, emit: bool) {
        self.event_emission = emit;
    }

    #[cfg(feature = "storage-event-control")]
    fn event_emission(&self) -> bool {
        self.event_emission
    }
}

impl<'a, C, T> SplitFlaggedStorage<'a, C, T> {
    #[cfg(feature = "storage-event-control")]
    fn emit_event(&self) -> bool {
        self.event_emission
    }

    #[cfg(not(feature = "storage-event-control"))]
    fn emit_event(&self) -> bool {
        true
    }
}

impl<'a, 'e, C, T> Join for &'a mut SplitFlaggedStorage<'e, C, T>
where
    C: Component,
    T: UnprotectedStorage<C>,
{
    type Mask = &'a BitSet;
    type Type = SplitFlaggedAccessMut<<T as UnprotectedStorage<C>>::AccessMut<'a>, C>;
    type Value = (&'a mut T, bool); // bool is event_emission

    // SAFETY: No unsafe code and no invariants to fulfill.
    unsafe fn open(self) -> (Self::Mask, Self::Value) {
        let event_emission = self.emit_event();
        (self.mask, (self.storage, event_emission))
    }

    unsafe fn get(v: &mut Self::Value, id: Index) -> Self::Type {
        // Note: comments copy-pasted from Storage Join impl

        // This is horribly unsafe. Unfortunately, Rust doesn't provide a way
        // to abstract mutable/immutable state at the moment, so we have to hack
        // our way through it.

        // # Safety
        //
        // See Join::get safety comments
        //
        // The caller is ensures any references returned are dropped before
        // any future calls to this method with the same Index value.
        //
        // Additionally, UnprotectedStorage::get_mut is required to return distinct
        // mutable references for distinct Index values and to not create references
        // internally that would alias with other mutable references produced
        // by calls with distinct Index values. Note, this isn't the same as `DistinctStorage`
        // which is stricter in that it requires internally mutable operations never alias for
        // distinct Index value (so it can be called in parallel).
        //
        // Thus, this will not create aliased mutable references.
        let (storage, event_emission) = v;
        let storage: *mut T = *storage as *mut T;
        let access = (*storage).get_access_mut(id);
        SplitFlaggedAccessMut {
            emit: *event_emission,
            id,
            access,
            phantom: PhantomData,
        }
    }
}

// SAFETY: This is safe because of the `DistinctStorage` guarantees.
#[cfg(feature = "parallel")]
unsafe impl<'a, 'e, C, T> ParJoin for &'a mut SplitFlaggedStorage<'e, C, T>
where
    C: Component,
    T: UnprotectedStorage<C>,
    T: Sync + DistinctStorage,
{
}

pub struct SplitFlaggedAccessMut<A, C> {
    emit: bool,
    id: Index,
    access: A,
    phantom: PhantomData<C>,
}

impl<A, C> Deref for SplitFlaggedAccessMut<A, C> {
    type Target = A;
    fn deref(&self) -> &Self::Target {
        &self.access
    }
}

impl<A, C> SplitFlaggedAccessMut<A, C> {
    /// Provides mutable access and emits a modification event.
    ///
    /// Note: Requiring the event collector/pool to have the same component type C doesn't
    /// guarantee completely that the [EventCollector]/[EventPool] and the [FlaggedAccessMut]
    /// are associated with the same source [UnsplitFlaggedStorage] but this does generally
    /// provide helpful resistance to misuse in most cases dealing with a single ecs world.
    /// If we want to guarantee this relation lifetime branding could be used instead.
    pub fn access<S: EventSink<C>>(&mut self, event_collector: &mut S) -> &mut A {
        if self.emit {
            event_collector.vec_mut().push(self.id);
        }
        &mut self.access
    }

    /// Like [Self::access] but skips emitting a modification event.
    pub fn access_untracked(&mut self) -> &mut A {
        &mut self.access
    }

    /// If you are going to be passing this around a lot it can be useful to combine it with
    /// the event collector so only one thing needs to be passed around for each wrapped component.
    ///
    /// Note: Requiring the event collector/pool to have the same component type C doesn't
    /// guarantee completely that the [EventCollector]/[EventPool] and the [FlaggedAccessMut]
    /// are associated with the same source [UnsplitFlaggedStorage] but this does generally
    /// provide helpful resistance to misuse in most cases dealing with a single ecs world.
    /// If we want to guarantee this relation lifetime branding could be used instead.
    pub fn with_collector<S: EventSink<C>>(
        self,
        event_collector: &mut S,
    ) -> FlaggedAccessMut<'_, A, C> {
        FlaggedAccessMut {
            emit: self.emit,
            id: self.id,
            access: self.access,
            phantom: self.phantom,
            events: event_collector.vec_mut(),
        }
    }
}

pub struct FlaggedAccessMut<'a, A, C> {
    emit: bool,
    id: Index,
    access: A,
    phantom: PhantomData<C>,
    events: &'a mut Vec<Index>,
}

impl<A, C> Deref for FlaggedAccessMut<'_, A, C> {
    type Target = A;
    fn deref(&self) -> &Self::Target {
        &self.access
    }
}

impl<'a, A, C> FlaggedAccessMut<'a, A, C> {
    /// Provides mutable access and emits a modification event.
    ///
    /// Note: Requiring the event collector/pool to have the same component type C doesn't
    /// guarantee completely that the [EventCollector]/[EventPool] and the [FlaggedAccessMut]
    /// are associated with the same source [UnsplitFlaggedStorage] but this does generally
    /// provide helpful resistance to misuse in most cases dealing with a single ecs world.
    /// If we want to guarantee this relation lifetime branding could be used instead.
    pub fn access<S: EventSink<C>>(&mut self) -> &mut A {
        if self.emit {
            self.events.push(self.id);
        }
        &mut self.access
    }

    /// Like [Self::access] but skips emitting a modification event.
    pub fn access_untracked(&mut self) -> &mut A {
        &mut self.access
    }
}
