use std::iter::Extend;
use std::ops::{Deref, DerefMut};

use shrev::{EventChannel, ReaderId};

use join::Join;
use storage::{MaskedStorage, Storage};
use world::{Component, Index};

/// `UnprotectedStorage`s that track modifications, insertions, and
/// removals of components.
pub trait Tracked {
    /// Event channel tracking modified/inserted/removed components.
    fn channel(&self) -> &EventChannel<ComponentEvent>;
    /// Mutable event channel tracking modified/inserted/removed components.
    fn channel_mut(&mut self) -> &mut EventChannel<ComponentEvent>;
}

pub enum ComponentEvent {
    Inserted(Index),
    Modified(Index),
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
}

impl<'e, T, D> Storage<'e, T, D>
where
    T: Component,
    T::Storage: Tracked,
    D: DerefMut<Target = MaskedStorage<T>>,
{
    /// Returns the event channel for insertions/removals/modifications of this storage's
    /// components.
    pub fn channel_mut(&mut self) -> &mut EventChannel<ComponentEvent> {
        unsafe { self.open() }.1.channel_mut()
    }

    /// Starts tracking component events.
    pub fn track(&mut self) -> ReaderId<ComponentEvent> {
        self.channel_mut().register_reader()
    }

    /// Flags an index.
    pub fn flag(&mut self, event: ComponentEvent) {
        self.channel_mut().single_write(event);
    }
}

