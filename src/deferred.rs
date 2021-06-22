use crate::{
    base::{Swap, Writer},
    traits::{Capture, StrongBuffer, WriterTag},
};

pub struct DeferredWriter<I, T = WriterTag<I>, C = Capture<I>> {
    // this will always be `Some` if nothing panics during swaps
    // the code in this crate won't panic, so only user code needs
    // to be checked
    swap: Option<Swap<C>>,
    writer: Writer<I, T>,
}

pub trait WaitingStrategy {}
impl WaitingStrategy for crate::strategy::saving::SavingStrategy {}
impl WaitingStrategy for crate::strategy::local_saving::LocalSavingStrategy {}
#[cfg(feature = "std")]
impl WaitingStrategy for crate::strategy::saving_park::SavingParkStrategy {}

impl<I: StrongBuffer> From<Writer<I>> for DeferredWriter<I>
where
    I::Strategy: WaitingStrategy,
{
    fn from(writer: Writer<I>) -> Self { Self::new_unchecked(writer) }
}

impl<I, T, C> DeferredWriter<I, T, C> {
    pub const fn new_unchecked(writer: Writer<I, T>) -> Self { Self { swap: None, writer } }
}

impl<I: StrongBuffer> DeferredWriter<I> {
    pub fn finish_swap(&mut self) -> &mut Writer<I> { self.finish_swap_with(|_| ()) }

    pub fn finish_swap_with<F: FnMut(&Writer<I>)>(&mut self, mut f: F) -> &mut Writer<I> {
        if let Some(swap) = self.swap.take() {
            let writer = &**self;
            let f = move || f(writer);
            writer.finish_buffer_swap_with(swap, f);
        }

        &mut self.writer
    }

    pub fn start_swap(&mut self) { self.swap = Some(unsafe { self.writer.start_buffer_swap() }); }

    pub fn into_inner(mut self) -> Writer<I> {
        self.finish_swap();
        self.writer
    }
}

impl<I, T, C> core::ops::Deref for DeferredWriter<I, T, C> {
    type Target = Writer<I, T>;

    fn deref(&self) -> &Self::Target { &self.writer }
}
