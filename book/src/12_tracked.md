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
    pub modify_reader_id: Option<ReaderId<ModifiedFlag>>,
    pub insert_reader_id: Option<ReaderId<InsertedFlag>>,
}

impl<'a> System<'a> for Sys {
    type SystemData = (
        ReadStorage<'a, Data>,
        WriteStorage<'a, SomeOtherData>,
    );
    
    fn run(&mut self, (data, some_other_data): Self::SystemData) {
        self.dirty.clear();
        
        data.populate_inserted(&mut self.insert_reader_id.as_mut().unwrap(), &mut self.dirty);
        
        // Note that we could use separate bitsets here, we only use one to simplify the example
        data.populate_modified(&mut self.modify_reader_id.as_mut().unwrap(), &mut self.dirty);
        
        for (d, other, _) in (&data, &mut some_other_data, &self.dirty) {
            // Mutate `other` based on the update data in `d`
        }
    }
    
    fn setup(&mut self, res: &mut Resources) {
        Self::SystemData::setup(res);
        let mut storage: WriteStorage<Data> = SystemData::fetch(&res);
        self.modify_reader_id = Some(storage.track_modified());
        self.insert_reader_id = Some(storage.track_inserted());
    }
}

```

There are three different event types that we can subscribe to:

* InsertedFlag - will be sent whan a component is added to the storage
* ModifiedFlag - will be sent when a component is fetched mutably from the storage
* RemovedFlag - will be sent when a component is removed from the storage

Note that because of how `ModifiedFlag` works, if you iterate mutably over a 
component storage using `Join`, all entities that are fetched by the `Join` will
be flagged as modified even if nothing was updated in them.

For `InsertedFlag` and `ModifiedFlag`, always check that the `Component` still exist
in the storage, and that the `Entity` is still alive. More on this below.

It is also quite important to process `RemovedFlag` events first if you are trying to
keep something in sync with the Specs `World`. This is because otherwise you might
miss synchronization for the following scenario:

* `System` runs.
* `Entity` with `Index` `X` get removed.
* `World::maintain` is called.
* `Entity` is created and get `Index` `X`.
* `System` runs.

At this point you will get both a `RemovedFlag` and an `InsertedFlag` in the same update.
If you process `InsertedFlag` events first you will see "X is new", and then `RemovedFlag`
 and see "X is removed". Now the synchronized state is invalid.

The following scenario will also need some consideration:

* `Entity` is created and get `Index` `X`.
* `Entity` with `Index` `X` get removed.
* `System` runs.

In this scenario, if you process `RemovedFlag` first, the removal event should be ignored 
because `X` will not have been synchronized before. It is because of this scenario you 
should always verify that the `Component` still exist in the storage, and that the `Entity` 
is still alive.
