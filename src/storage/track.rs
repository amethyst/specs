use std::iter::Extend;
use std::ops::{Deref, DerefMut};

use shrev::{EventChannel, ReaderId};

use join::Join;
use storage::{MaskedStorage, Storage};
use world::{Component, Index};

/// `UnprotectedStorage`s that track modifications, insertions, and
/// removals of components.
pub trait Tracked {
    /// Event channels tracking modified/inserted/removed components.
    fn channels(&self) -> &TrackChannels;
    /// Mutable event channels tracking modified/inserted/removed components.
    fn channels_mut(&mut self) -> &mut TrackChannels;
}

/// All three types of tracked modifications to components.
pub struct TrackChannels {
    /// Modifications event channel.
    ///
    /// Note: This does not include insertions, only when a component is changed
    /// after it has been added to an entity.
    pub modify: EventChannel<ModifiedFlag>,
    /// Insertions event channel.
    ///
    /// Note: Insertion events only occur when something inserts a component without
    /// there being a pre-existing component. If there is a pre-existing component
    /// for an entity then it will fire a modification event.
    pub insert: EventChannel<InsertedFlag>,
    /// Removed event channel.
    pub remove: EventChannel<RemovedFlag>,
}

impl TrackChannels {
    /// Creates a new structure that holds all types of component modification events.
    pub fn new() -> Self {
        Default::default()
    }
}

impl Default for TrackChannels {
    fn default() -> Self {
        TrackChannels {
            modify: EventChannel::new(),
            insert: EventChannel::new(),
            remove: EventChannel::new(),
        }
    }
}

impl<'e, T, D> Storage<'e, T, D>
where
    T: Component,
    T::Storage: Tracked,
    D: Deref<Target = MaskedStorage<T>>,
{
    /// Returns the event channel tracking modified components.
    pub fn channels(&self) -> &TrackChannels {
        unsafe { self.open() }.1.channels()
    }

    /// Returns the event channel tracking modified components.
    pub fn modified(&self) -> &EventChannel<ModifiedFlag> {
        &self.channels().modify
    }

    /// Returns the event channel tracking inserted components.
    pub fn inserted(&self) -> &EventChannel<InsertedFlag> {
        &self.channels().insert
    }

    /// Returns the event channel tracking removed components.
    pub fn removed(&self) -> &EventChannel<RemovedFlag> {
        &self.channels().remove
    }

    /// Reads events from the modified `EventChannel` and populates a structure using the events.
    pub fn populate_modified<E>(&self, reader_id: &mut ReaderId<ModifiedFlag>, value: &mut E)
    where
        E: Extend<Index>,
    {
        value.extend(self.modified().read(reader_id).map(|flag| *flag.as_ref()));
    }

    /// Reads events from the inserted `EventChannel` and populates a structure using the events.
    pub fn populate_inserted<E>(&self, reader_id: &mut ReaderId<InsertedFlag>, value: &mut E)
    where
        E: Extend<Index>,
    {
        value.extend(self.inserted().read(reader_id).map(|flag| *flag.as_ref()));
    }

    /// Reads events from the removed `EventChannel` and populates a structure using the events.
    pub fn populate_removed<E>(&self, reader_id: &mut ReaderId<RemovedFlag>, value: &mut E)
    where
        E: Extend<Index>,
    {
        value.extend(self.removed().read(reader_id).map(|flag| *flag.as_ref()));
    }
}

impl<'e, T, D> Storage<'e, T, D>
where
    T: Component,
    T::Storage: Tracked,
    D: DerefMut<Target = MaskedStorage<T>>,
{
    /// Returns all of the event channels for this component.
    pub fn channels_mut(&mut self) -> &mut TrackChannels {
        unsafe { self.open() }.1.channels_mut()
    }

    /// Returns the event channel tracking modified components mutably.
    pub fn modified_mut(&mut self) -> &mut EventChannel<ModifiedFlag> {
        &mut self.channels_mut().modify
    }

    /// Returns the event channel tracking inserted components mutably.
    pub fn inserted_mut(&mut self) -> &mut EventChannel<InsertedFlag> {
        &mut self.channels_mut().insert
    }

    /// Returns the event channel tracking removed components mutably.
    pub fn removed_mut(&mut self) -> &mut EventChannel<RemovedFlag> {
        &mut self.channels_mut().remove
    }

    /// Starts tracking modified events.
    pub fn track_modified(&mut self) -> ReaderId<ModifiedFlag> {
        self.modified_mut().register_reader()
    }

    /// Starts tracking inserted events.
    pub fn track_inserted(&mut self) -> ReaderId<InsertedFlag> {
        self.inserted_mut().register_reader()
    }

    /// Starts tracking removed events.
    pub fn track_removed(&mut self) -> ReaderId<RemovedFlag> {
        self.removed_mut().register_reader()
    }

    /// Flags an index as modified.
    pub fn flag_modified(&mut self, id: Index) {
        self.modified_mut().single_write(id.into());
    }

    /// Flags an index as inserted.
    pub fn flag_inserted(&mut self, id: Index) {
        self.inserted_mut().single_write(id.into());
    }

    /// Flags an index as removed.
    pub fn flag_removed(&mut self, id: Index) {
        self.removed_mut().single_write(id.into());
    }
}

macro_rules! flag {
    ( $( $name:ident ),* ) => {
        $(
            /// Flag with additional type safety against which kind of
            /// operations were done.
            #[derive(Copy, Clone, Debug, Eq, PartialEq)]
            pub struct $name(Index);
            impl Deref for $name {
                type Target = Index;
                fn deref(&self) -> &Self::Target {
                    &self.0
                }
            }

            impl AsRef<Index> for $name {
                fn as_ref(&self) -> &Index {
                    &self.0
                }
            }

            impl From<Index> for $name {
                fn from(flag: Index) -> Self {
                    $name(flag)
                }
            }
        )*
    }
}

// Separate types for type safety reasons.
flag!(ModifiedFlag, InsertedFlag, RemovedFlag);
