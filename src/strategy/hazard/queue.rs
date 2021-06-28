use core::{
    alloc::Layout,
    marker::PhantomData,
    ops::Deref,
    ptr::NonNull,
    sync::atomic::{AtomicBool, AtomicPtr, Ordering},
};
use std::alloc::{alloc, dealloc, handle_alloc_error};

#[repr(transparent)]
pub struct Queue<T, const SHOULD_DROP: bool> {
    head: AtomicPtr<QueueNodeInner<T>>,
}

#[repr(transparent)]
pub struct QueueNode<T, const SHOULD_DROP: bool> {
    ptr: NonNull<QueueNodeInner<T>>,
    mark: PhantomData<T>,
}

struct QueueNodeInner<T> {
    next: AtomicPtr<Self>,
    has_both: AtomicBool,
    value: T,
}

unsafe fn free_node<T>(ptr: *mut QueueNodeInner<T>) { dealloc(ptr.cast(), Layout::new::<QueueNodeInner<T>>()) }

impl<T, const SHOULD_DROP: bool> Drop for QueueNode<T, SHOULD_DROP> {
    fn drop(&mut self) {
        #[cold]
        #[inline(never)]
        unsafe fn free_node_slow<T>(ptr: *mut QueueNodeInner<T>) { free_node(ptr) }

        if unsafe {
            self.ptr
                .as_ref()
                .has_both
                .compare_exchange(true, false, Ordering::Release, Ordering::Relaxed)
                .is_err()
                && SHOULD_DROP
        } {
            unsafe { free_node_slow(self.ptr.as_ptr()) }
        }
    }
}

impl<T, const SHOULD_DROP: bool> Drop for Queue<T, SHOULD_DROP> {
    fn drop(&mut self) {
        if !SHOULD_DROP {
            return
        }

        let mut head = *self.head.get_mut();

        while !head.is_null() {
            let next = unsafe { *(*head).next.get_mut() };

            if unsafe {
                (*head)
                    .has_both
                    .compare_exchange(true, false, Ordering::Release, Ordering::Relaxed)
                    .is_err()
            } {
                unsafe { free_node(head) }
            }

            head = next;
        }
    }
}

impl<T, const SHOULD_DROP: bool> Queue<T, SHOULD_DROP> {
    pub const fn new() -> Self {
        Self {
            head: AtomicPtr::new(core::ptr::null_mut()),
        }
    }
}

impl<T, const SHOULD_DROP: bool> Deref for QueueNode<T, SHOULD_DROP> {
    type Target = T;
    fn deref(&self) -> &Self::Target { unsafe { &self.ptr.as_ref().value } }
}

impl<T: Send + Sync, const SHOULD_DROP: bool> Queue<T, SHOULD_DROP> {
    #[cold]
    #[inline(never)]
    fn alloc_slow(&self, default: T) -> QueueNode<T, SHOULD_DROP> {
        let ptr = unsafe { alloc(Layout::new::<QueueNodeInner<T>>()) };
        let ptr = ptr.cast::<QueueNodeInner<T>>();

        if ptr.is_null() {
            handle_alloc_error(core::alloc::Layout::new::<QueueNodeInner<T>>())
        }

        let mut head = self.head.load(Ordering::Relaxed);

        unsafe {
            ptr.write(QueueNodeInner {
                next: AtomicPtr::new(head),
                has_both: AtomicBool::new(true),
                value: default,
            })
        }

        loop {
            core::hint::spin_loop();
            if let Err(h) = self
                .head
                .compare_exchange(head, ptr, Ordering::Release, Ordering::Relaxed)
            {
                head = h
            } else {
                break
            }

            unsafe { (*ptr).next = AtomicPtr::new(head) }
        }

        QueueNode {
            ptr: unsafe { NonNull::new_unchecked(ptr) },
            mark: PhantomData,
        }
    }

    pub fn alloc(&self, default: T) -> QueueNode<T, SHOULD_DROP> {
        let mut head = self.head.load(Ordering::Acquire);

        unsafe {
            while !head.is_null() {
                if (*head)
                    .has_both
                    .compare_exchange(false, true, Ordering::Acquire, Ordering::Relaxed)
                    .is_ok()
                {
                    return QueueNode {
                        ptr: NonNull::new_unchecked(head),
                        mark: PhantomData,
                    }
                }

                head = (*head).next.load(Ordering::Relaxed);
            }
        }

        self.alloc_slow(default)
    }

    pub fn any<F: FnMut(&T) -> bool>(&self, mut finder: F) -> bool {
        let mut head = self.head.load(Ordering::Acquire);

        unsafe {
            while !head.is_null() {
                if (*head).has_both.load(Ordering::Acquire) && finder(&(*head).value) {
                    return true
                }

                head = (*head).next.load(Ordering::Relaxed);
            }
        }

        false
    }
}
