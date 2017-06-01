# Introduction

Welcome to the Specs book, an introduction into ECS and the Specs API.
This book is targeted to beginners, taking you through all the difficulties
from setting up to building and structuring a game with an ECS.

Specs is an ECS library allowing parallel system execution, with both low
overhead and high flexibility, different storage types and a type-level
system data model.

You didn't fully understand what this sentence was about? The next section
is for you! In case you already know what an ECS is, just skip it.

## What's an ECS?

The term **ECS** is a shorthand for Entity-component system. These are the three
core concepts. Each **entity** is associated with some **components**. Those entities and
components are processed by **systems**. This way, you have your data (components)
completely separated from the behaviour (systems). The entity just logically
groups components, so a `Velocity` is applied to the `Position` of the
same entity.

ECS is sometimes seen as a counterpart to Object-Oriented Programming. I wouldn't
say that's a hundred percent true, but let me give you some comparision here.

In OOP, your player might look like this (I've used Java for the example):

```java,ignore
public class Player extends Character {
    private final Transform transform;
    private final Inventory inventory;
}
```

There are several limitations here:

* There is no multiple inheritance, therefore OOP programming is limited to
  extending just `Character`; moreover, you have to think about "*Is* the player
  a collider or *has* it a collider?"
* You cannot easily extend the player with modding; all the
  attributes are hardcoded
* Imagine you want to add a NPC, which looks like this:

```java,ignore
public class Npc extends Character {
    private final Transform transform;
    private final Inventory inventory;
    private final boolean isFriendly;
}
```

Now you have the stuff duplicated; you would have to write mostly identical code for
your player and the NPC, even though e.g. they both share a transform.

This is where the ECS comes into play: Components are *associated* to entities;
you just insert some component, whenever you like. Also, you just define the functionality
once: One System, taking a `Force` and a `Mass` - it produces a `Velocity`.

In fact, an entity does not even own the components; it's just

```rust,ignore
struct Entity(u32, Generation);
```

where the first field is the id and the second one a generation, used to validate
if the entity hasn't been deleted. The components are stored separately, which
also has the advantage of being more cache efficient.

## Where to use an ECS?

In case you were looking for a general-purpose library for doing it
the data-oriented way, I have to disappoint you; it's not.
ECS libraries are best-suited for creating games; in fact, I've never
heard of anybody using an ECS for something else.

---

Okay, now that you were given a rough overview, let's continue
to [Chapter 2][c2] where we'll build our first actual application with Specs.

[am]: https://www.amethyst.rs
[ra]: https://github.com/nikomatsakis/rayon
[c2]: ./02_hello_world.html
