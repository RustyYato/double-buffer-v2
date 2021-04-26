use std::boxed::Box;

use core::{
    marker::PhantomData,
    ops::Deref,
    ptr::NonNull,
    sync::atomic::{AtomicUsize, Ordering},
};

pub struct Thin<T: ?Sized> {
    inner: NonNull<ThinInner<T>>,
    drop: PhantomData<T>,
}

unsafe impl<T: Send + Sync + ?Sized> Send for Thin<T> {}
unsafe impl<T: Send + Sync + ?Sized> Sync for Thin<T> {}

#[repr(C)]
pub struct ThinInner<T: ?Sized> {
    count: AtomicUsize,
    value: T,
}

impl<T> ThinInner<T> {
    pub fn new(value: T) -> Self {
        Self {
            count: AtomicUsize::new(0),
            value,
        }
    }
}

impl<T> Thin<T> {
    pub fn new(value: T) -> Self { Box::new(ThinInner::new(value)).into() }
}

impl<T: ?Sized> Thin<T> {
    pub fn strong_count(&self) -> usize { unsafe { self.inner.as_ref().count.load(Ordering::Acquire) } }
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
        let count = unsafe { self.inner.as_ref().count.fetch_add(1, Ordering::Relaxed) };

        if count >= (isize::MAX as usize) {
            struct Abort;

            impl Drop for Abort {
                fn drop(&mut self) { panic!() }
            }

            let _abort = Abort;
            panic!();
        }

        Self {
            inner: self.inner,
            drop: PhantomData,
        }
    }
}

impl<T: ?Sized> Drop for Thin<T> {
    fn drop(&mut self) {
        let count = unsafe { self.inner.as_ref().count.fetch_sub(1, Ordering::Release) };
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
