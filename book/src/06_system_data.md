# System Data

## Accessing Entities

You want to create/delete entities from a system? There is
good news for you. You can just use `specs::data::Entities`
(which also is in prelude) to do that. It implements
`SystemData` so just put it 

> Don't confuse `specs::data::Entities` with `specs::entity::Entities`.
  While the latter one is the actual resource, the first one is a type
  definition for `Fetch<entity::Entities>`.

Please note that you may never write to these `Entities`, so only
use `Fetch`. Even though it's immutable, you can atomically create
and delete entities with it. Just use the `.create()` and `.delete()`
methods, respectively. After dynamic entity creation / deletion,
a call `World::maintain` is necessary in order to make the changes
persistent and delete associated components.

You cannot, however, easily build an entity
with associated components. For that, you have to write these component
storages and insert a component for your entity.
