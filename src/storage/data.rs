use shred::{Fetch, FetchMut, ResourceId, Resources, SystemData};

use {Component, EntitiesRes, Storage};
use storage::MaskedStorage;

/// A storage with read access.
pub type ReadStorage<'a, T> = Storage<'a, T, Fetch<'a, MaskedStorage<T>>>;

impl<'a, T> SystemData<'a> for ReadStorage<'a, T>
where
    T: Component,
{
    fn fetch(res: &'a Resources, id: usize) -> Self {
        Storage::new(res.fetch(0), res.fetch(id))
    }

    fn reads(id: usize) -> Vec<ResourceId> {
        vec![
            ResourceId::new::<EntitiesRes>(),
            ResourceId::new_with_id::<MaskedStorage<T>>(id),
        ]
    }

    fn writes(_: usize) -> Vec<ResourceId> {
        vec![]
    }
}

/// A storage with read and write access.
pub type WriteStorage<'a, T> = Storage<'a, T, FetchMut<'a, MaskedStorage<T>>>;

impl<'a, T> SystemData<'a> for WriteStorage<'a, T>
where
    T: Component,
{
    fn fetch(res: &'a Resources, id: usize) -> Self {
        Storage::new(res.fetch(0), res.fetch_mut(id))
    }

    fn reads(_: usize) -> Vec<ResourceId> {
        vec![ResourceId::new::<EntitiesRes>()]
    }

    fn writes(id: usize) -> Vec<ResourceId> {
        vec![ResourceId::new_with_id::<MaskedStorage<T>>(id)]
    }
}
