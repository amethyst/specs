use std::ops::{Deref, DerefMut};

use serde::{Deserialize, Serialize, Serializer};
use serde::ser::SerializeStruct;

use {Component, Entity, Index, Join, Storage, UnprotectedStorage};
use storage::MaskedStorage;

/// The error type returned
/// by [`Storage::merge`].
///
/// [`Storage::merge`]: struct.Storage.html#method.merge
#[derive(Debug)]
pub enum MergeError {
    /// Returned if there is no
    /// entity matching the specified offset.
    NoEntity(Index),
}

/// Structure of packed components with offsets of which entities they belong to.
/// Offsets define which entities the components correspond to, based on a list of entities
/// the packed data is sent in with.
///
/// If the list of entities is all entities in the world, then the offsets in the
/// packed data are the indices of the entities.
#[derive(Debug, Serialize, Deserialize)]
pub struct PackedData<T> {
    /// List of components.
    pub components: Vec<T>,
    /// Offsets used to get entities which correspond to the components.
    pub offsets: Vec<Index>,
}

impl<T> PackedData<T> {
    /// Modifies the data to match an entity list's length for merging.
    pub fn pair_truncate<'a>(&mut self, entities: &'a [Entity]) {
        self.truncate(entities.len());
    }
    /// Truncates the length of components and offsets.
    pub fn truncate(&mut self, length: usize) {
        self.components.truncate(length);
        self.offsets.truncate(length);
    }
}

impl<'e, T, D> Serialize for Storage<'e, T, D>
where
    T: Component + Serialize,
    D: Deref<Target = MaskedStorage<T>>,
{
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        use hibitset::BitSetLike;

        // Serializes the storage in a format of PackedData<T>
        let (bitset, storage) = self.open();
        // Serialize a struct that has 2 fields
        let mut structure = serializer.serialize_struct("PackedData", 2)?;
        let mut components: Vec<&T> = Vec::new();
        let mut offsets: Vec<u32> = Vec::new();
        for index in bitset.iter() {
            offsets.push(index);
            let component = unsafe { storage.get(index) };
            components.push(component);
        }

        structure.serialize_field("offsets", &offsets)?;
        structure.serialize_field("components", &components)?;
        structure.end()
    }
}

impl<'e, T, D> Storage<'e, T, D>
where
    T: Component + Deserialize<'e>,
    D: DerefMut<Target = MaskedStorage<T>>,
{
    /// Merges a list of components into the storage.
    ///
    /// The list of entities will be used as the base for the offsets of the packed data.
    ///
    /// e.g.
    /// ```rust,ignore
    ///let list = vec![Entity(0, 1), Entity(1, 1), Entity(2, 1)];
    ///let packed = PackedData { offsets: [0, 2], components: [ ... ] };
    ///storage.merge(&list, packed);
    /// ```
    /// Would merge the components at offset 0 and 2, which would be `Entity(0, 1)` and
    /// `Entity(2, 1)` while ignoring
    /// `Entity(1, 1)`.
    ///
    /// Note:
    /// The entity list should be at least the same size as the packed data. To make sure,
    /// you can call `packed.pair_truncate(&entities)`.
    /// If the entity list is larger than the packed data then those entities are ignored.
    ///
    /// Packed data should also be sorted in ascending order of offsets.
    /// If this is deserialized from data received from serializing a storage it will be
    /// in ascending order.
    pub fn merge<'a>(
        &'a mut self,
        entities: &'a [Entity],
        mut packed: PackedData<T>,
    ) -> Result<(), MergeError> {
        for (component, offset) in packed.components.drain(..).zip(packed.offsets.iter()) {
            match entities.get(*offset as usize) {
                Some(entity) => {
                    self.insert(*entity, component);
                }
                None => {
                    return Err(MergeError::NoEntity(*offset));
                }
            }
        }
        Ok(())
    }
}
