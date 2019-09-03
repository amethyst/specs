use std::ops::{Deref, DerefMut};

use shrev::{EventChannel, ReaderId};

use crate::{
    join::Join,
    storage::{MaskedStorage, Storage},
    world::{Component, Index},
};

/// `UnprotectedStorage`s that track modifications, insertions, and
/// removals of components.
pub trait Tracked {
    /// Event channel tracking modified/inserted/removed components.
    fn channel(&self) -> &EventChannel<ComponentEvent>;
    /// Mutable event channel tracking modified/inserted/removed components.
    fn channel_mut(&mut self) -> &mut EventChannel<ComponentEvent>;

    /// Controls the events signal emission.
    /// When this is set to false the events modified/inserted/removed are
    /// not emitted.
    #[cfg(feature = "storage-event-control")]
    fn set_event_emission(&mut self, emit: bool);

    /// Returns the actual state of the event emission.
    #[cfg(feature = "storage-event-control")]
    fn event_emission(&self) -> bool;
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
/// Component storage events received from a `FlaggedStorage` or any storage
/// that implements `Tracked`.
pub enum ComponentEvent {
    /// An insertion event, note that a modification event will be triggered if
    /// the entity already had a component and had a new one inserted.
    Inserted(Index),
    /// A modification event, this will be sent any time a component is accessed
    /// mutably so be careful with joins over `&mut storages` as it could
    /// potentially flag all of them.
    Modified(Index),
    /// A removal event.
    Removed(Index),
}

impl<'e, T, D> Storage<'e, T, D>
where
    T: Component,
    T::Storage: Tracked,
    D: Deref<Target = MaskedStorage<T>>,
{
    /// Returns the event channel tracking modified components.
    pub fn channel(&self) -> &EventChannel<ComponentEvent> {
        unsafe { self.open() }.1.channel()
    }

    /// Returns the actual state of the event emission.
    #[cfg(feature = "storage-event-control")]
    pub fn event_emission(&self) -> bool {
        unsafe { self.open() }.1.event_emission()
    }
}

impl<'e, T, D> Storage<'e, T, D>
where
    T: Component,
    T::Storage: Tracked,
    D: DerefMut<Target = MaskedStorage<T>>,
{
    /// Returns the event channel for insertions/removals/modifications of this
    /// storage's components.
    pub fn channel_mut(&mut self) -> &mut EventChannel<ComponentEvent> {
        unsafe { self.open() }.1.channel_mut()
    }

    /// Starts tracking component events. Note that this reader id should be
    /// used every frame, otherwise events will pile up and memory use by
    /// the event channel will grow waiting for this reader.
    pub fn register_reader(&mut self) -> ReaderId<ComponentEvent> {
        self.channel_mut().register_reader()
    }

    /// Flags an index with a `ComponentEvent`.
    pub fn flag(&mut self, event: ComponentEvent) {
        self.channel_mut().single_write(event);
    }

    /// Controls the events signal emission.
    /// When this is set to false the events modified/inserted/removed are
    /// not emitted.
    #[cfg(feature = "storage-event-control")]
    pub fn set_event_emission(&mut self, emit: bool) {
        unsafe { self.open() }.1.set_event_emission(emit);
    }
}
