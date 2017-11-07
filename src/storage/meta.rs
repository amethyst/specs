
use Index;

pub trait Metadata<T>: Default {
    /// Grabs the specific metadata immutably without borrowing the storage.
    fn meta<S: 'static>(&self) -> &S { panic!("No metadata was set for this component"); }
    /// Grabs the specific metadata mutably without borrowing the storage.
    fn mut_meta<S: 'static>(&mut self) -> &mut S { panic!("No metadata was set for this component"); }

    // These methods shouldn't normally be used, mainly just for plumbing purposes where you disassociate the
    // `T::Storage` from its `T::Metadata`.
    fn clean<F>(&mut self, _: &F)
    where
        F: Fn(Index) -> bool { }
    fn get(&self, _: Index, _: &T) { }
    fn get_mut(&mut self, _: Index, _: &mut T) { }
    fn insert(&mut self, _: Index, _: &T) { }
    fn remove(&mut self, _: Index, _: &T) { }
}

impl<T> Metadata<T> for () { }

macro_rules! tuple_storage {
    ( $( $index:tt => $arg:ident, )* ) => {
        impl<T, $( $arg ),*> Metadata<T> for ( $( $arg, )* )
            where $( $arg: Metadata<T> + Default + 'static ),*
        {
            fn meta<S>(&self) -> &S
                where S: 'static
            {
                use std::any::TypeId;
                let s_type = TypeId::of::<S>();
                $(
                    if s_type == TypeId::of::<$arg>() {
                        unsafe {
                            return &*(&self.$index as *const $arg as *const S)
                        }
                    }
                )*

                panic!("Storage does not exist for this component")
            }
            fn mut_meta<S>(&mut self) -> &mut S
                where S: 'static
            {
                use std::any::TypeId;
                let s_type = TypeId::of::<S>();
                $(
                    if s_type == TypeId::of::<$arg>() {
                        unsafe {
                            return &mut *(&mut self.$index as *mut $arg as *mut S)
                        }
                    }
                )*

                panic!("Storage does not exist for this component")
            }

            fn clean<CF>(&mut self, f: &CF)
                where
                    CF: Fn(Index) -> bool {
                $( self.$index.clean(&f); )*
            }
            fn get(&self, id: Index, value: &T) {
                $( self.$index.get(id, value); )*
            }
            fn get_mut(&mut self, id: Index, value: &mut T) {
                $( self.$index.get_mut(id, value); )*
            }
            fn insert(&mut self, id: Index, value: &T) {
                $( self.$index.insert(id, value); )*
            }
            fn remove(&mut self, id: Index, value: &T) {
                $( self.$index.remove(id, value); )*
            }
        }
    }
}

tuple_storage!( 0 => A, );
tuple_storage!( 0 => A, 1 => B, );
tuple_storage!( 0 => A, 1 => B, 2 => C, );
tuple_storage!( 0 => A, 1 => B, 2 => C, 3 => D, );
tuple_storage!( 0 => A, 1 => B, 2 => C, 3 => D, 4 => E, );
tuple_storage!( 0 => A, 1 => B, 2 => C, 3 => D, 4 => E, 5 => F, );
tuple_storage!( 0 => A, 1 => B, 2 => C, 3 => D, 4 => E, 5 => F, 6 => G, );
tuple_storage!( 0 => A, 1 => B, 2 => C, 3 => D, 4 => E, 5 => F, 6 => G, 7 => H, );
tuple_storage!( 0 => A, 1 => B, 2 => C, 3 => D, 4 => E, 5 => F, 6 => G, 7 => H, 8 => I, );
tuple_storage!( 0 => A, 1 => B, 2 => C, 3 => D, 4 => E, 5 => F, 6 => G, 7 => H, 8 => I, 9 => J, );
tuple_storage!( 0 => A, 1 => B, 2 => C, 3 => D, 4 => E, 5 => F, 6 => G, 7 => H, 8 => I, 9 => J, 10 => K, );
tuple_storage!( 0 => A, 1 => B, 2 => C, 3 => D, 4 => E, 5 => F, 6 => G, 7 => H, 8 => I, 9 => J, 10 => K, 11 => L, );
