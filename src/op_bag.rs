use core::ops::Deref;
use std::vec::Vec;

use crate::traits::Operation;

pub struct OpBag<O> {
    operations: Vec<O>,
    applied: usize,
}

impl<O> OpBag<O> {
    pub const fn new() -> Self {
        Self {
            operations: Vec::new(),
            applied: 0,
        }
    }

    pub fn applied(&self) -> usize { self.applied }

    pub(crate) fn apply<B: ?Sized>(&mut self, buffer: &mut B)
    where
        O: Operation<B>,
    {
        for operation in self.operations.drain(..self.applied) {
            operation.apply_final(buffer);
        }

        self.applied = 0;

        for operation in self.operations.iter_mut() {
            self.applied += 1;
            operation.apply(buffer);
        }
    }

    pub fn push(&mut self, op: O) { self.operations.push(op); }

    pub fn reserve(&mut self, additional: usize) { self.operations.reserve(additional) }
}

impl<O> Deref for OpBag<O> {
    type Target = [O];

    fn deref(&self) -> &Self::Target { &self.operations }
}
