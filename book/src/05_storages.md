# Storages

Specs contains a bunch of different storages, all built and optimized for
different use cases. But let's see some basics first.

## Storage basics

What you specify in a component `impl`-block is an `UnprotectedStorage`.
Each `UnprotectedStorage` exposes an unsafe getter which does not
perform any checks whether the requested index for the component is valid
(the id of an entity is the index of its component). To allow checking them
and speeding up iteration, we have something called hierarchical bitsets,
provided by [`hibitset`](https://github.com/slide-rs/hibitset).

> **Note:** In case you don't know anything about bitsets,
  you can safely skip the following section about it. Just keep
  in mind that we have some mask which tracks for
  which entities a component exists.

How does it speed up the iteration? A hierarchical bitset is essentially
a multi-layer bitset, where each upper layer "summarizes" multiple bits
of the underlying layers. That means as soon as one of the underlying
bits is `1`, the upper one also becomes `1`, so that we can skip a whole
range of indices if an upper bit is `0` in that section. In case it's `1`,
we go down by one layer and perform the same steps again (it currently
has 4 layers).

## Storage overview

Here a list of the storages with a short description and a link
to the corresponding heading.

|Storage Type         |Description               |Optimized for                 |
|:-------------------:|--------------------------|------------------------------|
| [`BTreeStorage`]    | Works with a `BTreeMap`  | no particular case           |
| [`DenseVecStorage`] | Uses a redirection table | fairly often used components |
| [`HashMapStorage`]  | Uses a `HashMap`         | rare components              |
| [`NullStorage`]     | Can flag entities        | doesn't depend on rarity     |
| [`VecStorage`]      | Uses a sparse `Vec`      | commonly used components     |

[`BTreeStorage`]: #btreestorage
[`DenseVecStorage`]: #densevecstorage
[`HashMapStorage`]: #hashmapstorage
[`NullStorage`]: #nullstorage
[`VecStorage`]: #vecstorage

## `BTreeStorage`

It works using a `BTreeMap` and it's meant to be the default storage
in case you're not sure which one to pick, because it fits all scenarios
fairly well.

## `DenseVecStorage`

This storage uses two `Vec`s, one containing the actual data and the other
one which provides a mapping from the entity id to the index for the data vec
(it's a redirection table). This is useful when your component is bigger
than a `usize` because it consumes less RAM.

## `HashMapStorage`

This should be used for components which are associated to very few entities,
because it provides a lower insertion cost and is packed together more tightly.
You should not use it for frequently used components, because the hashing cost would definitely
be noticeable.

## `NullStorage`

As already described in the overview, the `NullStorage` does itself
only contain a user-defined ZST (=Zero Sized Type; a struct with no data in it,
like `struct Synced;`).
Because it's wrapped in a so-called `MaskedStorage`, insertions and deletions
modify the mask, so it can be used for flagging entities (like in this example
for marking an entity as `Synced`, which could be used to only synchronize
some of the entities over the network).

## `VecStorage`

This one has only one vector (as opposed to the `DenseVecStorage`). It
just leaves uninitialized gaps where we don't have any component.
Therefore it would be a waste of memory to use this storage for
rare components, but it's best suited for commonly used components
(like transform values).


## `RudyStorage` (Experimental)

There is an experimental `RudyStorage`, which can be enabled with the `rudy` feature
flag. It uses [the `rudy` crate][ru], the Rust implementation of [Judy Arrays][ja].

[ja]: http://judy.sourceforge.net
[ru]: https://crates.io/crates/rudy

Its benefits are not clear yet.
