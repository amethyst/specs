use crossbeam::sync::TreiberStack;

use {Component, Entity, World};

trait LazyUpdateInternal: Send + Sync {
    fn update(self: Box<Self>, world: &World);
}

impl<F> LazyUpdateInternal for F
where
    F: FnOnce(&World) + Send + Sync + 'static,
{
    fn update(self: Box<Self>, world: &World) {
        self(world);
    }
}

/// Lazy updates can be used for world updates
/// that need to borrow a lot of resources
/// and as such should better be done at the end.
/// They work lazily in the sense that they are
/// dispatched when calling `world.maintain()`.
/// Please note that the provided methods take `&self`
/// so there's no need to fetch `LazyUpdate` mutably.
/// This resource is added to the world by default.
#[derive(Default)]
pub struct LazyUpdate {
    stack: TreiberStack<Box<LazyUpdateInternal>>,
}

impl LazyUpdate {
    /// Lazily inserts a component for an entity.
    ///
    /// ## Examples
    ///
    /// ```
    /// # use specs::*;
    /// #
    /// struct Pos(f32, f32);
    ///
    /// impl Component for Pos {
    ///     type Storage = VecStorage<Self>;
    /// }
    ///
    /// struct InsertPos;
    ///
    /// impl<'a> System<'a> for InsertPos {
    ///     type SystemData = (Entities<'a>, Fetch<'a, LazyUpdate>);
    ///
    ///     fn run(&mut self, (ent, lazy): Self::SystemData) {
    ///         let a = ent.create();
    ///         lazy.insert(a, Pos(1.0, 1.0));
    ///     }
    /// }
    /// ```
    pub fn insert<C>(&self, e: Entity, c: C)
    where
        C: Component + Send + Sync,
    {
        self.execute(move |world| {
            world.write::<C>().insert(e, c);
        });
    }

    /// Lazily inserts components for entities.
    ///
    /// ## Examples
    ///
    /// ```
    /// # use specs::*;
    /// #
    /// struct Pos(f32, f32);
    ///
    /// impl Component for Pos {
    ///     type Storage = VecStorage<Self>;
    /// }
    ///
    /// struct InsertPos;
    ///
    /// impl<'a> System<'a> for InsertPos {
    ///     type SystemData = (Entities<'a>, Fetch<'a, LazyUpdate>);
    ///
    ///     fn run(&mut self, (ent, lazy): Self::SystemData) {
    ///         let a = ent.create();
    ///         let b = ent.create();
    ///
    ///         lazy.insert_all(vec![(a, Pos(3.0, 1.0)), (b, Pos(0.0, 4.0))]);
    ///     }
    /// }
    /// ```
    pub fn insert_all<C, I>(&self, iter: I)
    where
        C: Component + Send + Sync,
        I: IntoIterator<Item = (Entity, C)> + Send + Sync + 'static,
    {
        self.execute(move |world| {
            let mut storage = world.write::<C>();
            for (e, c) in iter {
                storage.insert(e, c);
            }
        });
    }

    /// Lazily removes a component.
    ///
    /// ## Examples
    ///
    /// ```
    /// # use specs::*;
    /// #
    /// struct Pos;
    ///
    /// impl Component for Pos {
    ///     type Storage = VecStorage<Self>;
    /// }
    ///
    /// struct RemovePos;
    ///
    /// impl<'a> System<'a> for RemovePos {
    ///     type SystemData = (Entities<'a>, Fetch<'a, LazyUpdate>);
    ///
    ///     fn run(&mut self, (ent, lazy): Self::SystemData) {
    ///         for entity in ent.join() {
    ///             lazy.remove::<Pos>(entity);
    ///         }
    ///     }
    /// }
    /// ```
    pub fn remove<C>(&self, e: Entity)
    where
        C: Component + Send + Sync,
    {
        self.execute(move |world| {
            world.write::<C>().remove(e);
        });
    }

    /// Lazily executes a closure with world access.
    ///
    /// ## Examples
    ///
    /// ```
    /// # use specs::*;
    /// #
    /// struct Pos;
    ///
    /// impl Component for Pos {
    ///     type Storage = VecStorage<Self>;
    /// }
    ///
    /// struct Execution;
    ///
    /// impl<'a> System<'a> for Execution {
    ///     type SystemData = (Entities<'a>, Fetch<'a, LazyUpdate>);
    ///
    ///     fn run(&mut self, (ent, lazy): Self::SystemData) {
    ///         for entity in ent.join() {
    ///             lazy.execute(move |world| {
    ///                 if world.is_alive(entity) {
    ///                     println!("Entity {:?} is alive.", entity);
    ///                 }
    ///             });
    ///         }
    ///     }
    /// }
    /// ```
    pub fn execute<F>(&self, f: F)
    where
        F: FnOnce(&World) + 'static + Send + Sync,
    {
        self.stack.push(Box::new(f));
    }

    pub(super) fn maintain(&mut self, world: &World) {
        let lazy = &mut self.stack;

        while let Some(l) = lazy.pop() {
            l.update(world);
        }
    }
}

impl Drop for LazyUpdate {
    fn drop(&mut self) {
        // TODO: remove as soon as leak is fixed in crossbeam
        while self.stack.pop().is_some() {}
    }
}
