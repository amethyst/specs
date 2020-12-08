use std::{
    marker::PhantomData,
    ops::{Deref, DerefMut},
};

use hibitset::BitSetLike;

use crate::{
    storage::{ComponentEvent, DenseVecStorage, Tracked, TryDefault, UnprotectedStorage},
    world::{Component, Index},
};

use shrev::EventChannel;

/// Wrapper storage that tracks modifications, insertions, and removals of
/// components through an `EventChannel`, in a similar manner to `FlaggedStorage`.
///
/// Unlike `FlaggedStorage`, this storage uses a wrapper type for mutable
/// accesses that only emits modification events when the component is actually
/// used mutably. This means that simply performing a mutable join or calling
/// `WriteStorage::get_mut` will not, by itself, trigger a modification event.
pub struct DerefFlaggedStorage<C, T = DenseVecStorage<C>> {
    channel: EventChannel<ComponentEvent>,
    storage: T,
    #[cfg(feature = "storage-event-control")]
    event_emission: bool,
    phantom: PhantomData<C>,
}

impl<C, T> DerefFlaggedStorage<C, T> {
    #[cfg(feature = "storage-event-control")]
    fn emit_event(&self) -> bool {
        self.event_emission
    }

    #[cfg(not(feature = "storage-event-control"))]
    fn emit_event(&self) -> bool {
        true
    }
}

impl<C, T> Default for DerefFlaggedStorage<C, T>
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

impl<C: Component, T: UnprotectedStorage<C>> UnprotectedStorage<C> for DerefFlaggedStorage<C, T> {
    type AccessMut<'a> where T: 'a = FlaggedAccessMut<'a, <T as UnprotectedStorage<C>>::AccessMut<'a>, C>;

    unsafe fn clean<B>(&mut self, has: B)
    where
        B: BitSetLike,
    {
        self.storage.clean(has);
    }

    unsafe fn get(&self, id: Index) -> &C {
        self.storage.get(id)
    }

    unsafe fn get_mut(&mut self, id: Index) -> Self::AccessMut<'_> {
        let emit = self.emit_event();
        FlaggedAccessMut {
            channel: &mut self.channel,
            emit,
            id,
            access: self.storage.get_mut(id),
            phantom: PhantomData,
        }
    }

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

impl<C, T> Tracked for DerefFlaggedStorage<C, T> {
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

pub struct FlaggedAccessMut<'a, A, C> {
    channel: &'a mut EventChannel<ComponentEvent>,
    emit: bool,
    id: Index,
    access: A,
    phantom: PhantomData<C>,
}

impl<'a, A, C> Deref for FlaggedAccessMut<'a, A, C>
    where A: Deref<Target = C>
{
    type Target = C;
    fn deref(&self) -> &Self::Target { self.access.deref() }
}

impl<'a, A, C> DerefMut for FlaggedAccessMut<'a, A, C>
    where A: DerefMut<Target = C>
{
    fn deref_mut(&mut self) -> &mut Self::Target {
        if self.emit {
            self.channel.single_write(ComponentEvent::Modified(self.id));
        }
        self.access.deref_mut()
    }
}
