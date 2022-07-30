// TODO: promote to the whole crate
#![deny(unsafe_op_in_unsafe_fn)]
//! Stand in for core::cell::SyncUnsafeCell since that is still unstable.
//!
//! TODO: Remove when core::cell::SyncUnsafeCell is stabilized

use core::cell::UnsafeCell;
use core::ops::{Deref, DerefMut};

#[repr(transparent)]
pub struct SyncUnsafeCell<T: ?Sized>(pub UnsafeCell<T>);

// SAFETY: Proper synchronization is left to the user of the unsafe `get` call.
// `UnsafeCell` itself doesn't implement `Sync` to prevent accidental mis-use.
unsafe impl<T: ?Sized + Sync> Sync for SyncUnsafeCell<T> {}

impl<T> SyncUnsafeCell<T> {
    pub fn new(value: T) -> Self {
        Self(UnsafeCell::new(value))
    }

    pub fn as_cell_of_slice(slice: &[Self]) -> &SyncUnsafeCell<[T]> {
        // SAFETY: `T` has the same memory layout as `SyncUnsafeCell<T>`.
        unsafe { &*(slice as *const [Self] as *const SyncUnsafeCell<[T]>) }
    }

    pub fn as_slice_mut(slice: &mut [Self]) -> &mut [T] {
        // SAFETY: `T` has the same memory layout as `SyncUnsafeCell<T>` and we
        // have a mutable reference which means the `SyncUnsafeCell` can be
        // safely removed since we have exclusive access here.
        unsafe { &mut *(slice as *mut [Self] as *mut [T]) }
    }
}

impl<T: ?Sized> Deref for SyncUnsafeCell<T> {
    type Target = UnsafeCell<T>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl<T: ?Sized> DerefMut for SyncUnsafeCell<T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

impl<T: Default> Default for SyncUnsafeCell<T> {
    fn default() -> Self {
        Self::new(Default::default())
    }
}
