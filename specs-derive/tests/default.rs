use specs::Component;

#[derive(Component)]
pub struct Foo;

static_assertions::assert_type_eq_all!(<Foo as Component>::Storage, ::specs::storage::DenseVecStorage<Foo>);
