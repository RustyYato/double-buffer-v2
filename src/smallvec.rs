use std::{
    marker::PhantomData,
    mem::{ManuallyDrop, MaybeUninit},
};

pub union SmallVec<T, const N: usize> {
    array: ManuallyDrop<Array<T, N>>,
    heap: Heap<T>,
}

#[repr(C)]
struct Array<T, const N: usize> {
    len: usize,
    items: MaybeUninit<[T; N]>,
}

#[repr(C)]
struct Heap<T> {
    cap: usize,
    len: usize,
    ptr: *const T,
    mark: PhantomData<T>,
}

impl<T> Copy for Heap<T> {}
impl<T> Clone for Heap<T> {
    fn clone(&self) -> Self { *self }
}

impl<T, const N: usize> Default for SmallVec<T, N> {
    fn default() -> Self { Self::new() }
}

impl<T, const N: usize> Drop for SmallVec<T, N> {
    fn drop(&mut self) {
        unsafe {
            if self.heap.cap > N {
                Vec::from_raw_parts(self.heap.ptr as *mut T, self.heap.len, self.heap.cap);
            } else {
                let ptr = self.array.items.as_mut_ptr().cast::<T>();
                std::ptr::slice_from_raw_parts_mut(ptr, self.array.len).drop_in_place();
            }
        }
    }
}

impl<T, const N: usize> SmallVec<T, N> {
    pub const fn new() -> Self {
        Self {
            array: ManuallyDrop::new(Array {
                len: 0,
                items: MaybeUninit::uninit(),
            }),
        }
    }

    pub fn len(&self) -> usize {
        unsafe {
            if self.heap.cap > N {
                self.heap.len
            } else {
                self.array.len
            }
        }
    }

    unsafe fn vec_mut(&mut self) -> ManuallyDrop<Vec<T>> {
        ManuallyDrop::new(Vec::from_raw_parts(
            self.heap.ptr as *mut T,
            self.heap.len,
            self.heap.cap,
        ))
    }

    fn reserve_one(&mut self) {
        unsafe {
            if self.array.len == N {
                let mut vec = ManuallyDrop::new(Vec::<T>::with_capacity(
                    N.checked_mul(2).expect("Could not allocate a the heap vector"),
                ));
                let array = &mut *self.array;
                vec.as_mut_ptr()
                    .copy_from_nonoverlapping(array.items.as_mut_ptr().cast(), array.len);
                self.heap = Heap {
                    cap: N * 2,
                    len: N,
                    ptr: vec.as_mut_ptr(),
                    mark: PhantomData,
                }
            } else if self.heap.cap > N && self.heap.cap.wrapping_sub(1) == self.heap.len {
                self.vec_mut().reserve(1);
            }
        }
    }

    pub fn push(&mut self, value: T) {
        self.reserve_one();

        if unsafe { self.heap.cap > N } {
            unsafe {
                let mut vec = self.vec_mut();
                vec.push(value);
                self.heap.len += 1;
                self.heap.cap = vec.capacity();
                self.heap.ptr = vec.as_mut_ptr();
            }
        } else {
            unsafe {
                let array = &mut *self.array;
                array.items.as_mut_ptr().cast::<T>().add(array.len).write(value);
                array.len += 1;
            }
        }
    }

    pub fn retain<F: FnMut(&mut T) -> bool>(&mut self, mut f: F) {
        let (ptr, len) = unsafe {
            if self.heap.cap > N {
                (self.heap.ptr as *mut T, &mut self.heap.len)
            } else {
                let array = &mut *self.array;
                (array.items.as_mut_ptr().cast(), &mut array.len)
            }
        };

        unsafe {
            let mut del = 0;
            {
                let len = *len;
                let v = core::slice::from_raw_parts_mut(ptr, len);
                let v = &mut v[..len];

                for i in 0..len {
                    if !f(&mut v[i]) {
                        del += 1;
                    } else {
                        let ptr = v.as_mut_ptr();
                        ptr.add(i).swap(ptr.add(i - del))
                        // core::mem::swap(&mut *ptr.add(i), &mut *ptr.add(i - del))
                    }
                }
            }
            if del > 0 {
                *len -= del;
                core::ptr::slice_from_raw_parts_mut(ptr.add(*len), del).drop_in_place();
            }
        }
    }

    #[cfg(test)]
    fn as_slice(&self) -> &[T] {
        unsafe {
            if self.heap.cap > N {
                &*core::ptr::slice_from_raw_parts(self.heap.ptr, self.heap.len)
            } else {
                &(*self.array.items.as_ptr())[..self.array.len]
            }
        }
    }
}

#[test]
fn test() {
    let mut vec = SmallVec::<_, 16>::new();
    vec.push(0);
    vec.push(1);
    vec.push(2);
    vec.push(3);

    assert_eq!(vec.as_slice(), [0, 1, 2, 3]);

    vec.retain(|x| *x % 2 == 0);

    assert_eq!(vec.as_slice(), [0, 2]);
}
