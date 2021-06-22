use core::{cell::Cell, marker::PhantomData, ops::Deref, ptr::NonNull};
use std::boxed::Box;

pub struct Thin<T: ?Sized> {
    inner: NonNull<ThinInner<T>>,
    drop: PhantomData<T>,
}

#[repr(C)]
pub struct ThinInner<T: ?Sized> {
    count: Cell<usize>,
    value: T,
}

impl<T> ThinInner<T> {
    pub fn new(value: T) -> Self {
        Self {
            count: Cell::new(0),
            value,
        }
    }
}

impl<T> Thin<T> {
    pub fn new(value: T) -> Self { Box::new(ThinInner::new(value)).into() }
}

impl<T: ?Sized> Thin<T> {
    pub fn strong_count(&self) -> usize { unsafe { self.inner.as_ref().count.get() } }
}

impl<T: ?Sized> From<Box<ThinInner<T>>> for Thin<T> {
    fn from(thin: Box<ThinInner<T>>) -> Self {
        let inner = unsafe { NonNull::new_unchecked(Box::into_raw(thin)) };
        Self {
            inner,
            drop: PhantomData,
        }
    }
}

impl<T: ?Sized> Clone for Thin<T> {
    fn clone(&self) -> Self {
        unsafe {
            let count = &self.inner.as_ref().count;
            count.set(
                count
                    .get()
                    .checked_add(1)
                    .expect("tried to clone a local thin too many times"),
            );
        }

        Self {
            inner: self.inner,
            drop: PhantomData,
        }
    }
}

impl<T: ?Sized> Drop for Thin<T> {
    fn drop(&mut self) {
        let count = unsafe {
            let count = &self.inner.as_ref().count;
            let old = count.get();
            count.set(old.wrapping_sub(1));
            old
        };
        if count == 0 {
            unsafe {
                Box::from_raw(self.inner.as_ptr());
            }
        }
    }
}

impl<T: ?Sized> Deref for Thin<T> {
    type Target = T;

    fn deref(&self) -> &Self::Target { unsafe { &self.inner.as_ref().value } }
}
