use shred::{Fetch, FetchMut, ResourceId, Resources, SystemData};

use storage::{MaskedStorage, Storage};
use world::{Component, EntitiesRes};

/// A storage with read access.
///
/// This is just a type alias for a fetched component storage.
///
/// The main functionality it provides is listed in the following,
/// however make sure to also check out the documentation for the
/// respective methods on `Storage`.
///
/// ## Joining storages
///
/// `&ReadStorage` implements `Join`, which allows to do
/// something like this:
///
/// ```
/// # use specs::prelude::*;
/// #
/// # struct Pos; impl Component for Pos { type Storage = VecStorage<Self>; }
/// # struct Vel; impl Component for Vel { type Storage = VecStorage<Self>; }
/// #
/// # let mut world = World::new(); world.register::<Pos>(); world.register::<Vel>();
/// # let pos_storage = world.read::<Pos>();
/// # let vel_storage = world.read::<Vel>();
/// (&pos_storage, &vel_storage).join()
/// # ;
/// ```
///
/// This joins the position and the velocity storage, which means it only
/// iterates over the components of entities that have both a position
/// **and** a velocity.
///
/// ## Retrieving single components
///
/// If you have an entity (for example because you stored it before
/// or because you're joining over `Entities`), you can get a single
/// component by calling `Storage::get`:
///
/// ```
/// # use specs::prelude::*;
/// # #[derive(Debug, PartialEq)]
/// # struct Pos; impl Component for Pos { type Storage = VecStorage<Self>; }
/// # #[derive(Debug, PartialEq)]
/// # struct Vel; impl Component for Vel { type Storage = VecStorage<Self>; }
/// #
/// # let mut world = World::new(); world.register::<Pos>(); world.register::<Vel>();
/// let entity1 = world.create_entity()
///     .with(Pos)
///     .build();
/// let entity2 = world.create_entity()
///     .with(Vel)
///     .build();
///
/// # let pos_storage = world.read::<Pos>();
/// # let vel_storage = world.read::<Vel>();
/// assert_eq!(pos_storage.get(entity1), Some(&Pos));
/// assert_eq!(pos_storage.get(entity2), None);
///
/// assert_eq!(vel_storage.get(entity1), None);
/// assert_eq!(vel_storage.get(entity2), Some(&Vel));
/// ```
///
/// ## Usage as `SystemData`
///
/// `ReadStorage` implements `SystemData` which allows you to
/// fetch it inside a system by simply adding it to the tuple:
///
/// ```
/// # use specs::prelude::*;
/// #[derive(Debug)]
/// struct Pos {
///     x: f32,
///     y: f32,
/// }
///
/// impl Component for Pos {
///     type Storage = VecStorage<Self>;
/// }
///
/// struct Sys;
///
/// impl<'a> System<'a> for Sys {
///     type SystemData = (Entities<'a>, ReadStorage<'a, Pos>);
///
///     fn run(&mut self, (ent, pos): Self::SystemData) {
///         for (ent, pos) in (&*ent, &pos).join() {
///             println!("Entitiy with id {} has a position of {:?}", ent.id(), pos);
///         }
///     }
/// }
/// ```
///
/// These operations can't mutate anything; if you want to do
/// insertions or modify components, you need to use `WriteStorage`.
/// Note that you can also use `LazyUpdate` , which does insertions on
/// `World::maintain`. This allows more concurrency and is designed
/// to be used for entity initialization.
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
///
/// Additionally to what `ReadStorage` can do a storage with mutable access allows:
///
/// ## Retrieve components mutably
///
/// This works just like `Storage::get`, but returns a mutable reference:
///
/// ```
/// # use specs::prelude::*;
/// # #[derive(Debug, PartialEq)]
/// # struct Pos(f32); impl Component for Pos { type Storage = VecStorage<Self>; }
/// #
/// # let mut world = World::new(); world.register::<Pos>();
/// let entity = world.create_entity()
///     .with(Pos(2.0))
///     .build();
/// # let mut pos_storage = world.write::<Pos>();
///
/// assert_eq!(pos_storage.get_mut(entity), Some(&mut Pos(2.0)));
/// if let Some(pos) = pos_storage.get_mut(entity) {
///     *pos = Pos(4.5);
/// }
///
/// assert_eq!(pos_storage.get(entity), Some(&Pos(4.5)));
/// ```
///
/// ## Inserting and removing components
///
/// You can insert components using `Storage::insert` and remove them
/// again with `Storage::remove`.
///
/// ```
/// # use specs::prelude::*;
/// # use specs::storage::InsertResult;
/// # #[derive(Debug, PartialEq)]
/// # struct Pos(f32); impl Component for Pos { type Storage = VecStorage<Self>; }
/// #
/// # let mut world = World::new(); world.register::<Pos>();
/// let entity = world.create_entity()
///     .with(Pos(0.1))
///     .build();
/// # let mut pos_storage = world.write::<Pos>();
///
/// let res = pos_storage.insert(entity, Pos(4.0));
/// assert_eq!(res, InsertResult::Updated(Pos(0.1)));
/// ```
///
/// There's also an Entry-API similar to the one provided by
/// `std::collections::HashMap`.
///
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
