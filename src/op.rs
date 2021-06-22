#![forbid(unsafe_code)]

use crate::{
    base::Writer,
    deferred::{DeferredWriter, WaitingStrategy},
    traits::{Buffer, Capture, Operation, StrongBuffer, WriterTag},
};

use crate::op_list::OpList;

pub struct OpWriter<I, O, T = WriterTag<I>, C = Capture<I>> {
    writer: DeferredWriter<I, T, C>,
    ops: OpList<O>,
}

impl<I: StrongBuffer, O> From<Writer<I>> for OpWriter<I, O>
where
    I::Strategy: WaitingStrategy,
{
    fn from(writer: Writer<I>) -> Self { Self::new(writer.into()) }
}

impl<I, O, T, C> From<DeferredWriter<I, T, C>> for OpWriter<I, O, T, C> {
    fn from(writer: DeferredWriter<I, T, C>) -> Self { Self::new(writer) }
}

pub struct Operations<'a, O> {
    list: &'a mut OpList<O>,
}

impl<I, O, T, C> OpWriter<I, O, T, C> {
    pub const fn new(writer: DeferredWriter<I, T, C>) -> Self {
        Self {
            writer,
            ops: OpList::new(),
        }
    }

    pub fn as_mut_parts(&mut self) -> (&Writer<I, T>, Operations<'_, O>) {
        (&self.writer, Operations { list: &mut self.ops })
    }

    pub fn applied(&self) -> usize { self.ops.applied() }

    pub fn reserve(&mut self, additional: usize) { self.ops.reserve(additional) }

    pub fn push(&mut self, op: O) { self.ops.push(op) }

    pub fn ops(&self) -> &[O] { &self.ops }
}

impl<I: StrongBuffer, O: Operation<Buffer<I>>> OpWriter<I, O> {
    pub fn swap_buffers(&mut self) { self.swap_buffers_with(|_, _| ()) }

    pub fn swap_buffers_with<F: FnMut(&Writer<I>, Operations<'_, O>)>(&mut self, f: F) {
        let (writer, ops) = self.finish_swap_with(f);
        ops.apply(writer.get_mut());
        self.start_swap()
    }

    pub fn finish_swap(&mut self) -> (&mut Writer<I>, &mut OpList<O>) { self.finish_swap_with(|_, _| ()) }

    pub fn finish_swap_with<F: FnMut(&Writer<I>, Operations<'_, O>)>(
        &mut self,
        mut f: F,
    ) -> (&mut Writer<I>, &mut OpList<O>) {
        let mut ops = Operations { list: &mut self.ops };
        let f = move |writer: &_| f(writer, ops.by_ref());
        (self.writer.finish_swap_with(f), &mut self.ops)
    }

    pub fn start_swap(&mut self) { self.writer.start_swap() }

    pub fn into_raw_parts(self) -> (DeferredWriter<I>, OpList<O>) { (self.writer, self.ops) }
}

impl<I, O, T, C> Extend<O> for OpWriter<I, O, T, C> {
    fn extend<Iter: IntoIterator<Item = O>>(&mut self, iter: Iter) { self.ops.extend(iter) }
}

impl<I, O, T, C> core::ops::Deref for OpWriter<I, O, T, C> {
    type Target = Writer<I, T>;

    fn deref(&self) -> &Self::Target { &self.writer }
}

impl<O> Operations<'_, O> {
    pub fn applied(&self) -> usize { self.list.applied() }

    pub fn push(&mut self, op: O) { self.list.push(op); }

    pub fn reserve(&mut self, additional: usize) { self.list.reserve(additional) }

    pub fn by_ref(&mut self) -> Operations<'_, O> { Operations { list: &mut self.list } }
}

impl<O> Extend<O> for Operations<'_, O> {
    fn extend<T: IntoIterator<Item = O>>(&mut self, iter: T) { self.list.extend(iter) }
}

impl<O> core::ops::Deref for Operations<'_, O> {
    type Target = [O];

    fn deref(&self) -> &Self::Target { self.list }
}
