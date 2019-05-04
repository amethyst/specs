# Introduction

Welcome to The Specs Book, an introduction to [ECS] and the Specs API.
This book is targeted at beginners; guiding you through all the difficulties of
setting up, building, and structuring a game with an ECS.

[ECS]: https://en.wikipedia.org/wiki/Entity–component–system

Specs is an ECS library that allows parallel system execution, with both low
overhead and high flexibility, different storage types and a type-level
system data model. It is mainly used for games and simulations, where it allows
to structure code using composition over inheritance.

Additional documentation is available on `docs.rs`:

* [API documentation for Specs](https://docs.rs/specs)

There also is a reference-style documentation available here:

* [Specs reference](../reference/)

You don't yet know what an ECS is all about? The next section
is for you! In case you already know what an ECS is, just skip it.

## What's an ECS?

The term **ECS** is a shorthand for Entity-component system. These are the three
core concepts. Each **entity** is associated with some **components**. Those entities and
components are processed by **systems**. This way, you have your data (components)
completely separated from the behaviour (systems). An entity just logically
groups components; so a `Velocity` component can be applied to the `Position` component
of the same entity.

ECS is sometimes seen as a counterpart to Object-Oriented Programming. I wouldn't
say that's one hundred percent true, but let me give you some comparisons.

In OOP, your player might look like this (I've used Java for the example):

```java
public class Player extends Character {
    private final Transform transform;
    private final Inventory inventory;
}
```

There are several limitations here:

* There is either no multiple inheritance or it brings other problems with it,
  like [the diamond problem][dp]; moreover, you have to think about "*is* the player
  a collider or does it *have* a collider?"
* You cannot easily extend the player with modding; all the attributes are hardcoded.
* Imagine you want to add a NPC, which looks like this:

[dp]: https://en.wikipedia.org/wiki/Multiple_inheritance#The_diamond_problem

```java
public class Npc extends Character {
    private final Transform transform;
    private final Inventory inventory;
    private final boolean isFriendly;
}
```

Now you have stuff duplicated; you would have to write mostly identical code for
your player and the NPC, even though e.g. they both share a transform.

<img src="./images/entity-component.svg" alt="Entity-component relationship" width="20%" style="float:left;margin-right:15px" />

This is where ECS comes into play: Components are *associated* with entities; you can just insert components, whenever you like.
One entity may or may not have a certain component. You can see an `Entity` as an ID into component tables, as illustrated in the
diagram below. We could theoretically store all the components together with the entity, but that would be very inefficient;
you'll see how these tables work in [chapter 5].

This is how an `Entity` is implemented; it's just

```rust,ignore
struct Entity(u32, Generation);
```

where the first field is the id and the second one is the generation, used to check
if the entity has been deleted.

Here's another illustration of the relationship between components and entities. `Force`, `Mass` and `Velocity` are all components here.

[chapter 5]: ./05_storages.html

<img src="./images/component-tables.svg" alt="Component tables" width="100%"  />

`Entity 1` has each of those components, `Entity 2` only a `Force`, etc.

Now we're only missing the last character in ECS - the "S" for `System`. Whereas components and entities are purely data,
systems contain all the logic of your application. A system typically iterates over all entities that fulfill specific constraints,
like "has both a force and a mass". Based on this data a system will execute code, e.g. produce a velocity out of the force and the mass.
This is the additional advantage I wanted to point out with the `Player` / `Npc` example; in an ECS, you can simply add new attributes
to entities and that's also how you define behaviour in Specs (this is called [data-driven] programming).

[data-driven]: https://en.wikipedia.org/wiki/Data-driven_programming

<img src="./images/system.svg" alt="System flow" width="100%"  />

By simply adding a force to an entity that has a mass, you can make it move, because a `Velocity` will be produced for it.

## Where to use an ECS?

In case you were looking for a general-purpose library for doing things the data-oriented way, I have to disappoint you; there are none.
ECS libraries are best-suited for creating games or simulations, but they do not magically make your code more data-oriented.

---

Okay, now that you were given a rough overview, let's continue
to [Chapter 2][c2] where we'll build our first actual application with Specs.

[am]: https://www.amethyst.rs
[ra]: https://github.com/rayon-rs/rayon
[c2]: ./02_hello_world.html

