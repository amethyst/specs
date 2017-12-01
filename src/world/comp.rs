use mopa::Any;

use {Metadata, UnprotectedStorage};

/// Abstract component type.
/// Doesn't have to be Copy or even Clone.
///
/// ## Storages
///
/// Components are stored in separated collections for maximum
/// cache efficiency. The `Storage` associated type allows
/// to specify which collection should be used.
/// Depending on how many entities have this component and how
/// often it is accessed, you will want different storages.
///
/// The most common ones are `VecStorage` (use if almost every entity has that component),
/// `DenseVecStorage` (if you expect many entities to have the component) and
/// `HashMapStorage` (for very rare components).
///
/// ## Examples
///
/// ```
/// use specs::{Component, VecStorage};
///
/// pub struct Position {
///     pub x: f32,
///     pub y: f32,
/// }
///
/// impl Component for Position {
///     type Storage = VecStorage<Self>;
///     type Metadata = ();
/// }
/// ```
///
/// ```
/// use specs::{Component, DenseVecStorage};
///
/// pub enum Light {
///     // (Variants would have additional data)
///     Directional,
///     SpotLight,
/// }
///
/// impl Component for Light {
///     type Storage = DenseVecStorage<Self>;
///     type Metadata = ();
/// }
/// ```
///
/// ```
/// use specs::{Component, HashMapStorage};
///
/// pub struct Camera {
///     // In an ECS, the camera would not itself have a position;
///     // you would just attach a `Position` component to the same
///     // entity.
///     matrix: [f32; 16],
/// }
///
/// impl Component for Camera {
///     type Storage = HashMapStorage<Self>;
///     type Metadata = ();
/// }
/// ```
pub trait Component: Any + Sized {
    /// Associated storage type for this component.
    type Storage: UnprotectedStorage<Self> + Any + Send + Sync;
    /// External data observing the data being accessed and modified.
    type Metadata: Metadata<Self> + Send + Sync;
}

