use core::ops::Deref;
use std::vec::Vec;

use crate::traits::Operation;

const POISON_BIT: usize = (!0) ^ (!0 >> 1);

pub struct OpList<O> {
    operations: Vec<O>,
    applied: usize,
}

impl<O> OpList<O> {
    pub const fn new() -> Self {
        Self {
            operations: Vec::new(),
            applied: 0,
        }
    }

    pub fn applied(&self) -> usize { self.applied }

    pub fn apply<B: ?Sized>(&mut self, buffer: &mut B)
    where
        O: Operation<B>,
    {
        struct SetOnDrop<'a>(&'a mut usize, usize);

        impl Drop for SetOnDrop<'_> {
            fn drop(&mut self) { *self.0 = self.1 }
        }

        if self.applied & POISON_BIT == 0 {
            self.apply_final(buffer);
        }

        self.applied |= POISON_BIT;
        let mut set_on_drop = SetOnDrop(&mut self.applied, POISON_BIT);
        let applied = &mut set_on_drop.1;

        for operation in self.operations.iter_mut() {
            *applied += 1;
            operation.apply(buffer);
        }

        drop(set_on_drop);
        self.assume_no_panic();
    }

    pub fn assume_no_panic(&mut self) { self.applied &= !POISON_BIT; }

    #[inline(always)]
    fn apply_final<B: ?Sized>(&mut self, buffer: &mut B)
    where
        O: Operation<B>,
    {
        struct Fixer<'a, T> {
            begin: *mut T,
            curr: *mut T,
            len: usize,
            vec: &'a mut Vec<T>,
        }

        impl<T> Drop for Fixer<'_, T> {
            fn drop(&mut self) {
                unsafe {
                    self.begin.copy_from(self.curr, self.len);
                    self.vec.set_len(self.len);
                }
            }
        }

        let len = self.operations.len();
        let ptr = self.operations.as_mut_ptr();

        let mut fixer = Fixer {
            begin: ptr,
            curr: ptr,
            len,
            vec: &mut self.operations,
        };

        while self.applied != 0 {
            unsafe {
                self.applied -= 1;
                fixer.len -= 1;
                let ptr = fixer.curr;
                fixer.curr = fixer.curr.add(1);
                ptr.read().apply_final(buffer)
            }
        }
    }

    pub fn push(&mut self, op: O) { self.operations.push(op); }

    pub fn reserve(&mut self, additional: usize) { self.operations.reserve(additional) }
}

impl<O> Extend<O> for OpList<O> {
    fn extend<T: IntoIterator<Item = O>>(&mut self, iter: T) { self.operations.extend(iter) }
}

impl<O> Deref for OpList<O> {
    type Target = [O];

    fn deref(&self) -> &Self::Target { &self.operations }
}
