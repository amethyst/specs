# Advanced strategies for components

So, now we have a fairly good grasp of the basics of Specs, so this is 
where we start experimenting with more advanced patterns!

## Marker components

Say we want to add a drag force to only some entities that have velocity, but
let other entities move about freely without drag?

The most common way is to use a marker component for this. A marker component
is a component without any data that can be added to entities to "mark" them
for processing, and can then be used to narrow down result sets using `Join`.

Some code for the drag example to clarify:

```rust,ignore
#[derive(Component)]
#[storage(NullStorage)]
pub struct Drag;

#[derive(Component)]
pub struct Position {
    pub pos: [f32; 3],
}

#[derive(Component)]
pub struct Velocity {
    pub velocity: [f32; 3],
}

struct Sys {
    drag: f32,
}

impl<'a> System<'a> for Sys {
    type SystemData = (
        ReadStorage<'a, Drag>,
        ReadStorage<'a, Velocity>,
        WriteStorage<'a, Position>,
    );
    
    fn run(&mut self, (drag, velocity, mut position): Self::SystemData) {
        // Update positions with drag
        for (pos, vel, _) in (&mut position, &velocity, &drag) {
            pos += vel - self.drag * vel * vel;
        }
        // Update positions without drag
        for (pos, vel, _) in (&mut position, &velocity, !&drag) {
            pos += vel;
        }
    } 
}
```

Using `NullStorage` is recommended for marker components, since they don't contain
any data and as such will not consume any memory. This means we can represent them using 
only a bitset.

## Modeling entity relationships and hierarchy

A common use case where we need a relationship between entities, is having a third person 
camera following the player around. We can model this using a targeting component 
referencing the player entity.

A simple implementation might look something like this:

```rust,ignore

#[derive(Component)]
pub struct Target {
    target: Entity,
    offset: Vector3,
}

pub struct FollowTargetSys;

impl<'a> System<'a> for FollowTargetSys {
    type SystemData = (
        Entities<'a>,
        ReadStorage<'a, Target>,
        WriteStorage<'a, Transform>,
    );
    
    fn run(&mut self, (entity, target, transform): Self::SystemData) {
        for (entity, t) in (&*entity, &target) {
            let new_transform = transform.get(t.target).cloned().unwrap() + t.offset;
            *transform.get_mut(entity).unwrap() = new_transform;
        }
    }
}
```

We could also model this as a resource (more about that in the next section), but it could 
be useful to be able to have multiple entities following targets, so modeling this with
a component makes sense. This could in extension be used to model large scale hierarchical
structure (scene graphs). For a generic implementation of such a hierarchical system, check
out the crate `specs-hiearchy`.

## Entity targeting

Imagine we're building a team based FPS game, and we want to add a spectator mode, where the
spectator can pick a player to follow. In this scenario each player will have a camera defined 
that is following them around, and what we want to do is to pick the camera that
we should use to render the scene on the spectator screen.

The easiest way to deal with this problem is to have a resource with a target entity, that
we can use to fetch the actual camera entity.

```rust,ignore
pub struct ActiveCamera(Entity);

pub struct Render;

impl<'a> System<'a> for Render {
    type SystemData = (
        Read<'a, ActiveCamera>,
        ReadStorage<'a, Camera>,
        ReadStorage<'a, Transform>,
        ReadStorage<'a, Mesh>,
    );
    
    fn run(&mut self, (active_cam, camera, transform, mesh) : Self::SystemData) {
        let camera = camera.get(active_cam.0);
        let view_matrix = transform.get(active_cam.0).invert();
        // Set projection and view matrix uniforms
        for (mesh, transform) in (&mesh, &transform) {
            // Set world transform matrix
            // Render mesh
        }
    }
}
```

By doing this, whenever the spectator chooses a new player to follow, we simply change
what `Entity` is referenced in the `ActiveCamera` resource, and the scene will be
rendered from the viewpoint instead.
