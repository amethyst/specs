/// A simple trait for transition between the fetch and processing phases
/// of Specs systems.
pub trait Gate {
    /// Transition destination type.
    type Target;
    /// Actually pass the gate. This may involve waiting on a ticketed lock.
    fn pass(self) -> Self::Target;
}

macro_rules! gate {
    // use variables to indicate the arity of the tuple
    ($($from:ident),*) => {
        impl<$($from: Gate),*> Gate for ($($from,)*) {
            type Target = ($($from::Target,)*);
            #[allow(non_snake_case)]
            fn pass(self) -> Self::Target {
                let ($($from,)*) = self;
                ($($from.pass(),)*)
            }
        }
    }
}

gate!();
gate!(A);
gate!(A, B);
gate!(A, B, C);
gate!(A, B, C, D);
gate!(A, B, C, D, E);
gate!(A, B, C, D, E, F);
gate!(A, B, C, D, E, F, G);
gate!(A, B, C, D, E, F, G, H);
