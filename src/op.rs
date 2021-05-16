use crate::{
    base::{Buffer, Capture, Reader, Split, Swap, Writer},
    traits::{Operation, Strategy, StrongBuffer},
};

use crate::op_list::OpList;

pub struct OpWriter<I, O, T = <<I as StrongBuffer>::Strategy as Strategy>::WriterTag, C = Capture<I>> {
    // this will always be `Some` if nothing panics during swaps
    // the code in this crate won't panic, so only user code needs
    // to be checked
    swap: Option<Swap<C>>,
    writer: Writer<I, T>,
    ops: OpList<O>,
}

#[derive(Debug)]
pub struct PoisonError(());

pub trait WaitingStrategy {}
impl WaitingStrategy for crate::strategy::saving::SavingStrategy {}
#[cfg(feature = "std")]
impl WaitingStrategy for crate::strategy::saving_park::SavingParkStrategy {}

impl<I: StrongBuffer, O> From<Writer<I>> for OpWriter<I, O>
where
    I::Strategy: WaitingStrategy,
{
    fn from(writer: Writer<I>) -> Self {
        Self {
            ops: OpList::new(),
            writer,
            swap: None,
        }
    }
}

pub struct Operations<'a, O> {
    list: &'a mut OpList<O>,
}

impl<I: StrongBuffer, O> OpWriter<I, O> {
    pub fn reader(&self) -> Reader<I::Weak> { self.writer.reader() }

    pub fn get(&self) -> &Buffer<I> { self.writer.get() }

    pub fn split(&self) -> Split<'_, Buffer<I>> { self.writer.split() }
}

impl<I: StrongBuffer, O: Operation<Buffer<I>>> OpWriter<I, O> {
    pub fn swap_buffers(&mut self) { self.swap_buffers_with(|_, _| ()) }

    pub fn swap_buffers_with<F: FnMut(&Writer<I>, Operations<'_, O>)>(&mut self, mut f: F) {
        if let Some(swap) = self.swap.take() {
            let ops = &mut self.ops;
            let writer = &self.writer;
            let f = move || f(writer, Operations { list: ops });
            self.writer.finish_buffer_swap_with(swap, f);
        }
        self.ops.apply(self.writer.get_mut());
        self.swap = Some(unsafe { self.writer.start_buffer_swap() })
    }
}

impl<I, O, T, C> OpWriter<I, O, T, C> {
    pub fn applied(&self) -> usize { self.ops.applied() }

    pub fn reserve(&mut self, additional: usize) { self.ops.reserve(additional) }

    pub fn push(&mut self, op: O) { self.ops.push(op) }

    pub fn ops(&self) -> &[O] { &self.ops }
}

impl<I, O, T, C> Extend<O> for OpWriter<I, O, T, C> {
    fn extend<Iter: IntoIterator<Item = O>>(&mut self, iter: Iter) { self.ops.extend(iter) }
}

impl<O> Operations<'_, O> {
    pub fn applied(&self) -> usize { self.list.applied() }

    pub fn push(&mut self, op: O) { self.list.push(op); }

    pub fn reserve(&mut self, additional: usize) { self.list.reserve(additional) }
}

impl<O> Extend<O> for Operations<'_, O> {
    fn extend<T: IntoIterator<Item = O>>(&mut self, iter: T) { self.list.extend(iter) }
}

impl<O> core::ops::Deref for Operations<'_, O> {
    type Target = [O];

    fn deref(&self) -> &Self::Target { &self.list }
}
