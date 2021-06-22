use std::{boxed::Box, cell::UnsafeCell};

use core::{
    marker::PhantomData,
    ops::Deref,
    ptr::NonNull,
    sync::atomic::{AtomicUsize, Ordering},
};

pub struct Athin<T: ?Sized> {
    inner: NonNull<AthinInner<T>>,
    drop: PhantomData<T>,
}

unsafe impl<T: Send + Sync + ?Sized> Send for Athin<T> {}
unsafe impl<T: Send + Sync + ?Sized> Sync for Athin<T> {}

#[repr(C)]
pub struct AthinInner<T: ?Sized> {
    count: AtomicUsize,
    value: T,
}

impl<T> AthinInner<T> {
    pub fn new(value: T) -> Self {
        Self {
            count: AtomicUsize::new(0),
            value,
        }
    }
}

impl Athin<AtomicUsize> {
    pub(crate) fn dangling() -> Self {
        static mut VALUE: UnsafeCell<AthinInner<AtomicUsize>> = UnsafeCell::new(AthinInner {
            count: AtomicUsize::new(1),
            value: AtomicUsize::new(0),
        });

        static DANGLING: Athin<AtomicUsize> = Athin {
            drop: PhantomData,
            inner: unsafe { NonNull::new_unchecked(VALUE.get()) },
        };

        DANGLING.clone()
    }
}

impl<T> Athin<T> {
    pub fn new(value: T) -> Self { Box::new(AthinInner::new(value)).into() }
}

impl<T: ?Sized> Athin<T> {
    pub fn strong_count(&self) -> usize { unsafe { self.inner.as_ref().count.load(Ordering::Acquire) } }
}

impl<T: ?Sized> From<Box<AthinInner<T>>> for Athin<T> {
    fn from(thin: Box<AthinInner<T>>) -> Self {
        let inner = unsafe { NonNull::new_unchecked(Box::into_raw(thin)) };
        Self {
            inner,
            drop: PhantomData,
        }
    }
}

impl<T: ?Sized> Clone for Athin<T> {
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

impl<T: ?Sized> Drop for Athin<T> {
    fn drop(&mut self) {
        let count = unsafe { self.inner.as_ref().count.fetch_sub(1, Ordering::Release) };
        if count == 0 {
            unsafe {
                Box::from_raw(self.inner.as_ptr());
            }
        }
    }
}

impl<T: ?Sized> Deref for Athin<T> {
    type Target = T;

    fn deref(&self) -> &Self::Target { unsafe { &self.inner.as_ref().value } }
}
