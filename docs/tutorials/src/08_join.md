# Joining components

In the last chapter, we learned how to access resources using `SystemData`.
To access our components with it, we can just request a `ReadStorage` and use
`Storage::get` to retrieve the component associated to an entity. This works quite
well if you want to access a single component, but what if you want to
iterate over many components? Maybe some of them are required, others might
be optional and maybe there is even a need to exclude some components?
If we wanted to do that using only `Storage::get`, the code would become very ugly.
So instead we worked out a way to conveniently specify that. This concept is
known as "joining".

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

The returned entity value can also be used to get a component from a storage as usual.

## Optional components

The previous example will iterate over all entities that have all the components 
we need, but what if we want to iterate over an entity whether it has a component 
or not?

To do that, we can wrap the `Storage` with `maybe()`: it wraps the `Storage` in a 
`MaybeJoin` struct which, rather than returning a component directly, returns 
`None` if the component is missing and `Some(T)` if it's there.
   
```rust,ignore
for (pos, vel, mass) in 
    (&mut pos_storage, &vel_storage, (&mut mass_storage).maybe()).join() {
    println!("Processing entity: {:?}", ent);
    *pos += *vel;

    if let Some(mass) = mass {
        let x = *vel / 300_000_000.0;
        let y = 1 - x * x;
        let y = y.sqrt();
        mass.current = mass.constant / y;
    }
}
```

In this example we iterate over all entities with a position and a velocity and 
perform the calculation for the new position as usual. However, in case the entity
has a mass, we also calculate the current mass based on the velocity. 
Thus, mass is an **optional** component here.

**WARNING:** Do not have a join of only `MaybeJoin`s. Otherwise the join will iterate 
over every single index of the bitset. If you want a join with all `MaybeJoin`s, 
add an EntitiesRes to the join as well to bound the join to all entities that are alive.

### Manually fetching components with `Storage::get()`

Even though `join()`ing over `maybe()` should be preferred because it can optimize how entities are
iterated, it's always possible to fetch a component manually using `Storage::get()`
or `Storage::get_mut()`.
For example, say that you want to damage a target entity every tick, but only if
it has an `Health`: 

```rust,ignore
for (target, damage) in (&target_storage, &damage_storage).join() {
  
    let target_health: Option<&mut Health> = health_storage.get_mut(target.ent);
    if let Some(target_health) = target_health {
      target_health.current -= damage.value;      
    }
}
```

Even though this is a somewhat contrived example, this is a common pattern 
when entities interact.

## Excluding components

If you want to filter your selection by excluding all entities
with a certain component type, you can use the not operator (`!`)
on the respective component storage. Its return value is a unit (`()`).

```rust,ignore
for (ent, pos, vel, ()) in (
    &*entities,
    &mut pos_storage,
    &vel_storage,
    !&frozen_storage,
).join() {
    println!("Processing entity: {:?}", ent);
    *pos += *vel;
}
```

This will simply iterate over all entities that

* have a position
* have a velocity
* do not have a `Frozen` component

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
