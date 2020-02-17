# `FlaggedStorage` and modification events

In most games you will have many entities, but from frame to frame there will
usually be components that will only need to updated when something related is
modified.

To avoid a lot of unnecessary computation when updating components it
would be nice if we could somehow check for only those entities that are updated
and recalculate only those.

We might also need to keep an external resource in sync with changes to
components in Specs `World`, and we only want to propagate actual changes, not
do a full sync every frame.

This is where `FlaggedStorage` comes into play. By wrapping a component's actual
storage in a `FlaggedStorage`, we can subscribe to modification events, and
easily populate bitsets with only the entities that have actually changed.

Let's look at some code:

```rust,ignore
pub struct Data {
    [..]
}

impl Component for Data {
    type Storage = FlaggedStorage<Self, DenseVecStorage<Self>>;
}

#[derive(Default)]
pub struct Sys {
    pub dirty: BitSet,
    pub reader_id: Option<ReaderId<ComponentEvent>>,
}

impl<'a> System<'a> for Sys {
    type SystemData = (
        ReadStorage<'a, Data>,
        WriteStorage<'a, SomeOtherData>,
    );

    fn run(&mut self, (data, mut some_other_data): Self::SystemData) {
        self.dirty.clear();

        let events = data.channel().read(self.reader_id.as_mut().unwrap());

        // Note that we could use separate bitsets here, we only use one to
        // simplify the example
        for event in events {
            match event {
                ComponentEvent::Modified(id) | ComponentEvent::Inserted(id) => {
                    self.dirty.add(*id);
                }
                 // We don't need to take this event into account since
                 // removed components will be filtered out by the join;
                 // if you want to, you can use `self.dirty.remove(*id);`
                 // so the bit set only contains IDs that still exist
                 ComponentEvent::Removed(_) => (),
            }
        }

        for (d, other, _) in (&data, &mut some_other_data, &self.dirty).join() {
            // Mutate `other` based on the update data in `d`
        }
    }

    fn setup(&mut self, res: &mut Resources) {
        Self::SystemData::setup(res);
        self.reader_id = Some(
            WriteStorage::<Data>::fetch(&res).register_reader()
        );
    }
}
```

There are three different event types that we can receive:

- `ComponentEvent::Inserted` - will be sent when a component is added to the
  storage
- `ComponentEvent::Modified` - will be sent when a component is fetched mutably
  from the storage
- `ComponentEvent::Removed` - will be sent when a component is removed from the
  storage

## Gotcha: Iterating `FlaggedStorage` Mutably

Because of how `ComponentEvent` works, if you iterate mutably over a
component storage using `Join`, all entities that are fetched by the `Join` will
be flagged as modified even if nothing was updated in them.

For example, this will cause all `comps` components to be flagged as modified:

```rust,ignore
// **Never do this** if `comps` uses `FlaggedStorage`.
//
// This will flag all components as modified regardless of whether the inner
// loop actually modified the component.
for comp in (&mut comps).join() {
    // ...
}
```

Instead, you will want to either:

- Restrict the components mutably iterated over, for example by joining with a
  `BitSet` or another component storage.
- Iterating over the components use a `RestrictedStorage` and only fetch the
  component as mutable if/when needed.

## `RestrictedStorage`

If you need to iterate over a `FlaggedStorage` mutably and don't want every
component to be marked as modified, you can use a `RestrictedStorage` and only
fetch the component as mutable if/when needed.

```rust,ignore
for (entity, mut comp) in (&entities, &mut comps.restrict_mut()).join() {
    // Check whether this component should be modified, without fetching it as
    // mutable.
    if comp.get_unchecked().condition < 5 {
         let mut comp = comp.get_mut_unchecked();
         // ...
    }
}
```

## Start and Stop event emission

Sometimes you may want to perform some operations on the storage, but you don't
want that these operations produce any event.

You can use the function `storage.set_event_emission(false)` to suppress the
event writing for of any action. When you want to re activate them you can
simply call `storage.set_event_emission(true)`.

---

_See
[FlaggedStorage Doc](https://docs.rs/specs/latest/specs/struct.FlaggedStorage.html)
for more into._