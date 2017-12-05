
use std::ops::{Deref, DerefMut};

use hibitset::{BitSet};
use shrev::{EventChannel, EventReadData, ReaderId};

use {Component, Index, Join, MaskedStorage, Storage};

/// `UnprotectedStorage`s that track modifications, insertions, and
/// removals of components.
pub trait Tracked {
    /// Event channel tracking modified components.
    fn modified(&self) -> &EventChannel<ModifiedFlag>;
    /// Mutable event channel tracking modified components.
    fn modified_mut(&mut self) -> &mut EventChannel<ModifiedFlag>;
    /// Event channel tracking inserted components.
    fn inserted(&self) -> &EventChannel<InsertedFlag>;
    /// Mutable event channel tracking inserted components.
    fn inserted_mut(&mut self) -> &mut EventChannel<InsertedFlag>;
    /// Event channel tracking removed components.
    fn removed(&self) -> &EventChannel<RemovedFlag>;
    /// Mutable event channel tracking removed components.
    fn removed_mut(&mut self) -> &mut EventChannel<RemovedFlag>;

    /// Tracks component modified events.
    fn track_modified(&self) -> ReaderId<ModifiedFlag> {
        self.modified().register_reader()
    }
    /// Tracks component inserted events.
    fn track_inserted(&self) -> ReaderId<InsertedFlag> {
        self.inserted().register_reader()
    }
    /// Tracks component removed events.
    fn track_removed(&self) -> ReaderId<RemovedFlag> {
        self.removed().register_reader()
    }
    /// Tracks modified, inserted, and removed events.
    fn track(&self) -> (ReaderId<ModifiedFlag>, ReaderId<InsertedFlag>, ReaderId<RemovedFlag>) {
        (
            self.track_modified(),
            self.track_inserted(),
            self.track_removed(),
        )
    }
}

impl<'e, T, D> Storage<'e, T, D>
where
    T: Component,
    T::Storage: Tracked,
    D: Deref<Target = MaskedStorage<T>>,
{
    /// Returns the event channel tracking modified components.
    pub fn modified(&self) -> &EventChannel<ModifiedFlag> {
        self.open().1.modified()
    }

    /// Returns the event channel tracking inserted components.
    pub fn inserted(&self) -> &EventChannel<InsertedFlag> {
        self.open().1.inserted()
    }

    /// Returns the event channel tracking removed components.
    pub fn removed(&self) -> &EventChannel<RemovedFlag> {
        self.open().1.removed()
    }

    /// Starts tracking modified events.
    pub fn track_modified(&self) -> ReaderId<ModifiedFlag> {
        self.open().1.track_modified()
    }

    /// Starts tracking inserted events.
    pub fn track_inserted(&self) -> ReaderId<InsertedFlag> {
        self.open().1.track_inserted()
    }

    /// Starts tracking removed events.
    pub fn track_removed(&self) -> ReaderId<RemovedFlag> {
        self.open().1.track_removed()
    }

    /// Reads events from the modified `EventChannel` and populates a bitset using the events.
    pub fn populate_modified(&self, reader_id: &mut ReaderId<ModifiedFlag>, bitset: &mut BitSet) {
        self.modified().read(reader_id).populate(bitset);
    }

    /// Reads events from the inserted `EventChannel` and populates a bitset using the events.
    pub fn populate_inserted(&self, reader_id: &mut ReaderId<InsertedFlag>, bitset: &mut BitSet) {
        self.inserted().read(reader_id).populate(bitset);
    }

    /// Reads events from the removed `EventChannel` and populates a bitset using the events.
    pub fn populate_removed(&self, reader_id: &mut ReaderId<RemovedFlag>, bitset: &mut BitSet) {
        self.removed().read(reader_id).populate(bitset);
    }
}

impl<'e, T, D> Storage<'e, T, D>
where
    T: Component,
    T::Storage: Tracked,
    D: DerefMut<Target = MaskedStorage<T>>,
{
    /// Returns the event channel tracking modified components mutably.
    pub fn modified_mut(&mut self) -> &mut EventChannel<ModifiedFlag> {
        self.open().1.modified_mut()
    }

    /// Returns the event channel tracking inserted components mutably.
    pub fn inserted_mut(&mut self) -> &mut EventChannel<InsertedFlag> {
        self.open().1.inserted_mut()
    }

    /// Returns the event channel tracking removed components mutably.
    pub fn removed_mut(&mut self) -> &mut EventChannel<RemovedFlag> {
        self.open().1.removed_mut()
    }

    /// Flags an index as modified.
    pub fn flag_modified(&mut self, id: Index) {
        self.modified_mut().single_write(Flag::Flag(id).into());
    }

    /// Unflags an index as modified.
    pub fn unflag_modified(&mut self, id: Index) {
        self.modified_mut().single_write(Flag::Unflag(id).into());
    }

    /// Flags an index as inserted.
    pub fn flag_inserted(&mut self, id: Index) {
        self.inserted_mut().single_write(Flag::Flag(id).into());
    }

    /// Unflags an index as inserted.
    pub fn unflag_inserted(&mut self, id: Index) {
        self.inserted_mut().single_write(Flag::Unflag(id).into());
    }

    /// Flags an index as removed.
    pub fn flag_removed(&mut self, id: Index) {
        self.removed_mut().single_write(Flag::Flag(id).into());
    }

    /// Unflags an index as removed.
    pub fn unflag_removed(&mut self, id: Index) {
        self.removed_mut().single_write(Flag::Unflag(id).into());
    }
}

/// Event for flagging or unflagging an index.
#[derive(Clone, Copy)]
pub enum Flag {
    /// Flags an index.
    Flag(Index),
    /// Unflags an index.
    Unflag(Index),
}

macro_rules! flag {
    ( $( $name:ident ),* ) => {
        $( 
            /// Flag with additional type safety against which kind of
            /// operations were done.
            pub struct $name(Flag);
            impl Deref for $name {
                type Target = Flag;
                fn deref(&self) -> &Self::Target {
                    &self.0
                }
            }

            impl AsRef<Flag> for $name {
                fn as_ref(&self) -> &Flag {
                    &self.0
                }
            }

            impl From<Flag> for $name {
                fn from(flag: Flag) -> Self {
                    $name(flag)
                }
            }
        )*
    }
}

flag!(ModifiedFlag, InsertedFlag, RemovedFlag);

/// Clears and populates a structure with an event channel's contents.
pub trait Populate<T> {
    /// Clears and populates a structure.
    fn populate(self, t: &mut T);
}

impl<'a, F> Populate<BitSet> for EventReadData<'a, F>
where
    F: AsRef<Flag>,
{
    fn populate(self, bitset: &mut BitSet) {
        bitset.clear();

        let iterator = match self {
            EventReadData::Data(iterator) => iterator,
            EventReadData::Overflow(iterator, amount) => {
                eprintln!("Populating ring buffer overflowed {} times!", amount);
                iterator
            }
        };

        for item in iterator {
            let flag: &Flag = item.as_ref();
            match flag {
                &Flag::Flag(index) => bitset.add(index),
                &Flag::Unflag(index) => bitset.remove(index),
            };
        }
    }
}

