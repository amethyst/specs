# Joining components

In the last chapter, we learned how to access resources using `SystemData`.
To access our components with it, we can just request a `ReadStorage` and use
`Storage::get` to retrieve the component associated to an entity. This works quite
well if you want to access a single component, but what if you want to
iterate over many components? Maybe some of them are required, others might
be optional and maybe there is even a need to exclude some components?
If we wanted to do that using only `Storage::get`, the code would become very ugly.
So instead we worked out a way to conveniently specify that. This concept is
known as "Joining".

## Basic joining

We've already seen some basic examples of joining in the last chapters, for
example we saw how to join over two storages:

```rust,ignore
for (pos, vel) in (&mut pos_storage, &vel_storage).join() {
    *pos += *vel;
}
```

This simply iterates over the position and velocity components of
all entities that have both these components. That means all the
specified components are **required**.

Sometimes, we want not only get the components of entities,
but also the entity value themselves. To do that, we can simply join over
`&EntitiesRes`.

```rust,ignore
for (ent, pos, vel) in (&*entities, &mut pos_storage, &vel_storage).join() {
    println!("Processing entity: {:?}", ent);
    *pos += *vel;
}
```

## Optional components

If we iterate over the `&EntitiesRes` as shown above, we can simply
use the returned `Entity` values to get components from storages as usual.

```rust,ignore
for (ent, pos, vel) in (&*entities, &mut pos_storage, &vel_storage).join() {
    println!("Processing entity: {:?}", ent);
    *pos += *vel;
    
    let mass: Option<&mut Mass> = mass_storage.get_mut(ent);
    if let Some(mass) = mass {
        let x = *vel / 300_000_000.0;
        let y = 1 - x * x;
        let y = y.sqrt();
        mass.current = mass.constant / y;
    }
}
```

In this example we iterate over all entities with a position and a velocity
and perform the calculation for the new position as usual.
However, in case the entity has a mass, we also calculate the current
mass based on the velocity. Thus, mass is an **optional** component here.

## Excluding components

If you want to filter your selection by excluding all entities
with a certain component type, you can use the not operator (`!`)
on the respective component storage. Its return value is a unit (`()`).

```rust,ignore
for (ent, pos, vel, ()) in (
    &*entities,
    &mut pos_storage,
    &vel_storage,
    !&freezed_storage,
).join() {
    println!("Processing entity: {:?}", ent);
    *pos += *vel;
}
```

This will simply iterate over all entities that

* have a position
* have a velocity
* do not have a `Freezed` component

## How joining works

You can call `join()` on everything that implements the `Join` trait.
The method call always returns an iterator. `Join` is implemented for

* `&ReadStorage` / `&WriteStorage` (gives back a reference to the components)
* `&mut WriteStorage` (gives back a mutable reference to the components)
* `&EntitiesRes` (returns `Entity` values)
* bitsets

We think the last point here is pretty interesting, because
it allows for even more flexibility, as you will see in the next
section.

## Joining over bitsets

Specs is using `hibitset`, a library which provides layered bitsets
(those were part of Specs once, but it was decided that a separate
library could be useful for others).

These bitsets are used with the component storages to determine
which entities the storage provides a component value for. Also,
`Entities` is using bitsets, too. You can even create your
own bitsets and add or remove entity ids:

```rust,ignore
use hibitset::{BitSet, BitSetLike};

let mut bitset = BitSet::new();
bitset.add(entity1.id());
bitset.add(entity2.id());
```

`BitSet`s can be combined using the standard binary operators,
`&`, `|` and `^`. Additionally, you can negate them using `!`.
This allows you to combine and filter components in multiple ways.

---

This chapter has been all about looping over components; but we can do more
than sequential iteration! Let's look at some parallel code in the next
chapter.
