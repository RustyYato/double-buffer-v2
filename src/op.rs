use crate::{
    base::{Buffer, Capture, Split, Swap, Writer},
    traits::{Operation, Strategy, StrongBuffer},
};

use crate::op_bag::OpBag;

pub struct OpWriter<I, O, T = <<I as StrongBuffer>::Strategy as Strategy>::WriterTag, C = Capture<I>> {
    swap: Option<Swap<C>>,
    writer: Writer<I, T>,
    ops: OpBag<O>,
}

#[derive(Debug)]
pub struct PoisonError(());

pub trait WaitingStrategy {}
impl WaitingStrategy for crate::strategy::sync::SyncStrategy {}
#[cfg(feature = "std")]
impl WaitingStrategy for crate::strategy::park::ParkStrategy {}

impl<I: StrongBuffer, O: Operation<Buffer<I>>> From<Writer<I>> for OpWriter<I, O>
where
    I::Strategy: WaitingStrategy,
{
    fn from(mut writer: Writer<I>) -> Self {
        let swap = unsafe { writer.start_buffer_swap() };
        Self {
            ops: OpBag::new(),
            writer,
            swap: Some(swap),
        }
    }
}

impl<I: StrongBuffer, O> OpWriter<I, O> {
    pub fn get(&self) -> &Buffer<I> { self.writer.get() }

    pub fn split(&self) -> Split<'_, Buffer<I>> { self.writer.split() }
}

impl<I: StrongBuffer, O: Operation<Buffer<I>>> OpWriter<I, O> {
    pub fn swap_buffers(&mut self) { self.swap_buffers_with(|_, _| ()) }

    pub fn swap_buffers_with<F: FnMut(&Writer<I>, &mut OpBag<O>)>(&mut self, f: F) {
        #[cold]
        #[inline(never)]
        fn swap_buffers_fail() -> ! { panic!("Could not swap poisoned buffers") }

        match self.try_swap_buffers_with(f) {
            Ok(()) => (),
            Err(PoisonError(())) => swap_buffers_fail(),
        }
    }

    pub fn try_swap_buffers(&mut self) -> Result<(), PoisonError> { self.try_swap_buffers_with(|_, _| ()) }

    pub fn try_swap_buffers_with<F: FnMut(&Writer<I>, &mut OpBag<O>)>(&mut self, mut f: F) -> Result<(), PoisonError> {
        let swap = self.swap.take().ok_or(PoisonError(()))?;
        let ops = &mut self.ops;
        let writer = &self.writer;
        let f = move || f(writer, ops);
        self.writer.finish_buffer_swap_with(swap, f);
        self.ops.apply(self.writer.get_mut());
        self.revive_from_poisoned_unchecked();
        Ok(())
    }

    pub fn revive_from_poisoned_unchecked(&mut self) { self.swap = Some(unsafe { self.writer.start_buffer_swap() }) }
}

impl<I, O, T, C> OpWriter<I, O, T, C> {
    pub fn push(&mut self, op: O) { self.ops.push(op) }

    pub fn ops(&self) -> &[O] { &self.ops }
}

impl<I, O, T, C> Extend<O> for OpWriter<I, O, T, C> {
    fn extend<Iter: IntoIterator<Item = O>>(&mut self, iter: Iter) { self.ops.extend(iter) }
}
