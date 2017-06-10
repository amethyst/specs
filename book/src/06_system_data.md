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

You cannot, however, easily build an entity
with associated components. For that, you have to write to these component
storages and insert a component for your entity.
