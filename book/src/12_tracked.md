# `FlaggedStorage` and modification events

In most games you will have many entities, but from frame to frame there will
usually be components that will only need to updated when something related is
modified.

To avoid a lot of unnecessary computation when updating components it
would be nice if we could somehow check for only those entities that are updated
and recalculate only those.

We might also need to keep an external resource in sync with changes
to components in Specs `World`, and we only want to propagate actual changes, not
do a full sync every frame.

This is where `FlaggedStorage` comes into play. By wrapping a component's
actual storage in a `FlaggedStorage`, we can subscribe to modification events, and
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
        self.reader_id = Some(WriteStorage::<Data>::fetch(&res).register_reader());
    }
}
```

There are three different event types that we can receive:

* `ComponentEvent::Inserted` - will be sent when a component is added to the
  storage
* `ComponentEvent::Modified` - will be sent when a component is fetched mutably
  from the storage
* `ComponentEvent::Removed` - will be sent when a component is removed from the
  storage

Note that because of how `ComponentEvent` works, if you iterate mutably over a
component storage using `Join`, all entities that are fetched by the `Join` will
be flagged as modified even if nothing was updated in them.
