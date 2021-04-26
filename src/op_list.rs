use core::ops::Deref;
use std::vec::Vec;

use crate::traits::Operation;

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
        let applied = core::mem::take(&mut self.applied);
        for operation in self.operations.drain(..applied) {
            operation.apply_final(buffer);
        }

        for operation in self.operations.iter_mut() {
            self.applied += 1;
            operation.apply(buffer);
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
