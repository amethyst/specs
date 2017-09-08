# [DRAFT] System Data

## Accessing Entities

You want to create/delete entities from a system? There is
good news for you. You can just use `Entities` to do that.
It implements `SystemData` so just put it in your `SystemData` tuple.

> Don't confuse `specs::Entities` with `specs::EntitiesRes`.
  While the latter one is the actual resource, the other one is a type
  definition for `Fetch<entity::Entities>`.

Please note that you may never write to these `Entities`, so only
use `Fetch`. Even though it's immutable, you can atomically create
and delete entities with it. Just use the `.create()` and `.delete()`
methods, respectively. After dynamic entity creation / deletion,
a call `World::maintain` is necessary in order to make the changes
persistent and delete associated components.

## Adding and removing components

Adding or removing components can be done by modifying
either components storage directly with a `WriteStorage`
or lazily using the `LazyUpdate` resource.

```rust,ignore
use specs::{Component, Fetch, LazyUpdate, NullStorage, System, Entities, WriteStorage};

struct Stone;
impl Component for Stone {
    type Storage = NullStorage<Self>;
}

struct StoneCreator;
impl<'a> System<'a> for StoneCreator {
    type SystemData = (
        Entities<'a>,
        WriteStorage<'a, Stone>,
        Fetch<'a, LazyUpdate>
    );

    fn run(&mut self, (entities, stones, updater): Self::SystemData) {
        let stone = entities.create();

        // 1) Either we insert the component by writing to its storage
        stones.insert(stone, Stone);

        // 2) or we can lazily insert it with `LazyUpdate`
        updater.insert(stone, Stone);
    }
}
```

> **Note:** After using `LazyUpdate` a call to `World::maintain`
  is necessary to actually execute the changes.
