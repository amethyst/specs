# Introduction

Welcome to The Specs Book, an introduction to ECS and the Specs API.
This book is targeted at beginners; guiding you through all the difficulties of
setting up, building, and structuring a game with an ECS.

Specs is an ECS library that allows parallel system execution, with both low
overhead and high flexibility, different storage types and a type-level
system data model.

You didn't fully understand what that sentence was about? The next section
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

This is where the ECS comes into play: Components are *associated* with entities;
you just insert some component, whenever you like. Also, you only define the functionality
once: One System, taking a `Force` and a `Mass` - it produces a `Velocity`.

In fact, an entity does not even own the components; it's just

```rust,ignore
struct Entity(u32, Generation);
```

where the first field is the id and the second one is the generation, used to validate
if the entity hasn't been deleted. The components are stored separately, which
also has the advantage of being more cache efficient.

Another way you can think of an entity is that it just
groups together a group of components. We'll see how this works
in practice later - it allows us to just pick sets of components
which include both `A` and `B`.

## Where to use an ECS?

In case you were looking for a general-purpose library for doing it
the data-oriented way, I have to disappoint you; there is none.
ECS libraries are best-suited for creating games; in fact, I've never
heard of anybody using an ECS for something else.

---

Okay, now that you were given a rough overview, let's continue
to [Chapter 2][c2] where we'll build our first actual application with Specs.

[am]: https://www.amethyst.rs
[ra]: https://github.com/nikomatsakis/rayon
[c2]: ./02_hello_world.html
