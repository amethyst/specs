
# Specs Procedural Derive Macros

[![Build Status][bi]][bl] [![Crates.io][ci]][cl] [![Gitter][gi]][gl] ![MIT/Apache][li] [![Docs.rs][di]][dl]

[bi]: https://travis-ci.org/slide-rs/specs.svg?branch=master
[bl]: https://travis-ci.org/slide-rs/specs

[ci]: https://img.shields.io/crates/v/specs.svg
[cl]: https://crates.io/crates/specs/

[li]: https://img.shields.io/badge/license-Apache%202.0-blue.svg

[di]: https://docs.rs/specs/badge.svg
[dl]: https://docs.rs/specs/

[gi]: https://badges.gitter.im/slide-rs/specs.svg
[gl]: https://gitter.im/slide-rs/specs

## Component Grouping

In a couple situations you are required to perform some task on a bunch of different components, 
such as serialization or registering the component into the world. Tasks like these require either
dynamic dispatch through the use of trait objects or closures, but that can make you lose performance
and prevent some compiler optimizations. However, the alternative is a lot of boilerplate and isn't
very scalable if using third party code where they also require registering components similar to the
`World`.

In these situations it is useful to use a macro to reduce that boilerplate and make it less annoying
to add new components to the program. Component groups mark a bunch of different components that
you want to have some operation perform on all of them.

### Usage


Component groups use a procedural derive macro along with using attributes with the tag `group`.
Fields in the component group `struct` are by default components, alternatively they can be marked
as a subgroup using the `subgroup` attribute. Subgroups in a component group will try to behave the
same as if the components were in the parent group. The field names are also used as unique identifiers
for components in things like serialization.

Note: When using the `call` macro to use the component groups, it is necessary to use another method 
on subgroups for subgroups to behave correctly in the `call` macro.

```rust
#[derive(ComponentGroup)]
struct ExampleGroup {
    // The group defaults to just a component.
    //
    // The field name "component1" will be used as an
    // unique identifier.
    component1: Component1,

    // Component grouping comes with built in support
    // for serialization and deserialization with `serde`
    // usage
    #[group(serialize)]
    component2: Component2,

    #[group(id = "5")]
    component3: Component3,
}

#[derive(ComponentGroup)]
struct AnotherGroup {
    component4: Component4,

    // If you need a subgroup, then you need to
    // designate the fields that are subgroups.
    #[group(subgroup)]
    example_group: ExampleGroup,
}
```

When operated on a method, this component group will look similar to something like:
```rust
fn method<T>() { ... }
method::<Component1>();
method::<Component2>();
method::<Component3>();
method::<Component4>();
```

### Attributes

`#[group(subgroup)]`
Marks the field as a subgroup. Will attempt to behave similar to if the components nested in the subgroup
are in the parent group.

`#[group(serialize)]`
Marks the field as a serializable component or subgroup. All fields that are marked by this should implement
`Serialize` and `Deserialize`. The field name is used as the unique identifier for serializaion.

`#[group(id = "...")]`
Identifies a component id other than the default `0usize`.

### `call` Macro

Component groups also provide capability for external extension using the `call` macro.
