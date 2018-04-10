use std::ops::DerefMut;

use error::Result;
use storage::DistinctStorage;
use storage::MaskedStorage;
use storage::Storage;
use world::Component;
use world::Entity;
use world::Index;

impl<'e, T, D> Storage<'e, T, D>
where
    Self: DistinctStorage,
    T: Component,
    D: DerefMut<Target = MaskedStorage<T>>,
{
    /// Retrieves multiple components mutably.
    /// `iter` is assumed to be sorted, otherwise this method will panic.
    ///
    /// # Panics
    ///
    /// * if `iter` returns the entities out of order
    pub fn entries<I>(&mut self, iter: I) -> Entries<I::IntoIter, Self>
    where
        I: IntoIterator<Item = Entity>,
    {
        Entries {
            index: None,
            iter: iter.into_iter(),
            storage: self,
        }
    }
}

/// An iterator over the entries returned by `Storage::entries`.
pub struct Entries<'a, I, S: 'a> {
    index: Option<Index>,
    iter: I,
    storage: &'a mut S,
}

impl<'a, 'e, T, D, I> Iterator for Entries<'a, I, Storage<'e, T, D>>
where
    D: DerefMut<Target = MaskedStorage<T>>,
    I: Iterator<Item = Entity>,
    T: Component,
    T::Storage: DistinctStorage,
{
    type Item = Result<&'a mut T>;

    #[inline]
    fn next(&mut self) -> Option<Result<&'a mut T>> {
        let next: Entity = match self.iter.next() {
            Some(entity) => entity,
            None => return None,
        };

        match self.index {
            None => {}
            Some(index) => assert!(
                index < next.id(),
                "Entries must be in order and non-overlapping"
            ),
        }

        self.index = Some(next.id());

        unsafe {
            use std::mem::transmute;

            // This is allowed because of the guarantees by `DistinctStorage`.

            Some(transmute::<Result<&mut T>, Result<&'a mut T>>(
                self.storage.get_mut(next).ok_or_else(|| unimplemented!()),
            ))
        }
    }
}

pub trait FromEntries<'a, C: 'a> {
    fn from_entries<I>(entries: I) -> Self
    where
        I: Iterator<Item = Result<&'a mut C>>;
}

impl<'a, C: 'a> FromEntries<'a, C> for (Result<&'a mut C>, Result<&'a mut C>) {
    fn from_entries<I>(mut entries: I) -> Self
    where
        I: Iterator<Item = Result<&'a mut C>>,
    {
        (entries.next().unwrap(), entries.next().unwrap())
    }
}

impl<'a, C: 'a> FromEntries<'a, C> for Result<(&'a mut C, &'a mut C)> {
    fn from_entries<I>(mut entries: I) -> Self
    where
        I: Iterator<Item = Result<&'a mut C>>,
    {
        Ok((entries.next().unwrap()?, entries.next().unwrap()?))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use storage::VecStorage;
    use world::{Component, World};

    #[derive(Clone, Debug, PartialEq)]
    struct Comp(i32);

    impl Component for Comp {
        type Storage = VecStorage<Self>;
    }

    #[test]
    fn basic() {
        let mut world = World::new();
        world.register::<Comp>();

        let a = world.create_entity().with(Comp(1)).build();
        let b = world.create_entity().with(Comp(2)).build();
        let c = world.create_entity().build();
        let d = world.create_entity().with(Comp(3)).build();

        let mut storage = world.write::<Comp>();
        let mut entries = storage.entries(vec![a, b, c, d]);

        assert_eq!(*entries.next().unwrap().unwrap(), Comp(1));
        assert_eq!(*entries.next().unwrap().unwrap(), Comp(2));
        assert!(entries.next().unwrap().is_err());
        assert_eq!(*entries.next().unwrap().unwrap(), Comp(3));

        assert!(entries.next().is_none());
    }

    #[test]
    fn swap() {
        use std::mem::swap;

        let mut world = World::new();
        world.register::<Comp>();

        let a = world.create_entity().with(Comp(11)).build();
        let b = world.create_entity().with(Comp(22)).build();

        let mut storage = world.write::<Comp>();
        {
            let entries = storage.entries(vec![a, b]);
            let entries: Result<(&mut Comp, &mut Comp)> = FromEntries::from_entries(entries);
            let (a, b) = entries.unwrap();
            swap(a, b);
        }

        assert_eq!(storage.get(a), Some(&Comp(22)));
        assert_eq!(storage.get(b), Some(&Comp(11)));
    }
}
