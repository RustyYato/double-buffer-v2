use radium::Radium;

use core::sync::atomic::Ordering;

use crate::traits::{Buffer, Capture, CaptureError, Strategy, StrongBuffer, TrustedRadium};

use super::Reader;

pub struct Writer<I, T = <<I as StrongBuffer>::Strategy as Strategy>::WriterTag> {
    tag: T,
    inner: I,
}

pub struct Swap<C>(C);

pub struct Split<'a, B: ?Sized> {
    pub writer: &'a B,
    pub reader: &'a B,
}

pub struct SplitMut<'a, B: ?Sized> {
    pub writer: &'a mut B,
    pub reader: &'a B,
}

struct FinishSwapOnDrop<'a, S: Strategy> {
    strategy: &'a S,
    swap: &'a mut S::Capture,
}

impl<S: Strategy> Drop for FinishSwapOnDrop<'_, S> {
    fn drop(&mut self) {
        while !self.strategy.readers_have_exited(self.swap) {
            self.strategy.pause(&mut self.swap)
        }
    }
}

impl<I, T> Writer<I, T> {
    pub(crate) unsafe fn from_raw_parts(inner: I, tag: T) -> Self { Self { tag, inner } }
}

impl<I: StrongBuffer> Writer<I> {
    pub fn reader(&self) -> Reader<I::Weak> { Reader::new(&self.inner) }

    pub fn strategy(&self) -> &I::Strategy { &self.inner.strategy }

    pub fn get(&self) -> &Buffer<I> { self.split().writer }

    pub fn get_mut(&mut self) -> &mut Buffer<I> { self.split_mut().writer }

    pub fn split(&self) -> Split<'_, Buffer<I>> {
        let inner = &*self.inner;

        // `Reader` only read from `which`, and reads can't race
        // `Writer` only writes to `which` under a `&mut _`,
        // so that can't race with any reads or writes within `Writer`
        let which = unsafe { inner.which.load_unsync() };
        let (writer, reader) = unsafe { inner.raw.split(which) };

        Split { writer, reader }
    }

    pub fn split_mut(&mut self) -> SplitMut<'_, Buffer<I>> {
        let inner = &*self.inner;

        // `Reader` only read from `which`, and reads can't race
        // `Writer` only writes to `which` under a `&mut _`,
        // so that can't race with any reads or writes within `Writer`
        let which = unsafe { inner.which.load_unsync() };
        let (writer, reader) = unsafe { inner.raw.split_mut(which) };

        SplitMut { writer, reader }
    }

    pub fn try_swap_buffers(&mut self) -> Result<(), CaptureError<I>> {
        let swap = unsafe { self.try_start_buffer_swap()? };
        self.finish_buffer_swap(swap);
        Ok(())
    }

    pub fn swap_buffers(&mut self) {
        let swap = unsafe { self.start_buffer_swap() };
        self.finish_buffer_swap(swap);
    }

    pub fn try_swap_buffers_with<F: FnMut(&Self)>(&mut self, mut f: F) -> Result<(), CaptureError<I>> {
        let swap = unsafe { self.try_start_buffer_swap()? };
        let this = &*self;
        let f = move || f(this);
        this.finish_buffer_swap_with(swap, f);
        Ok(())
    }

    pub fn swap_buffers_with<F: FnMut(&Self)>(&mut self, mut f: F) {
        let swap = unsafe { self.start_buffer_swap() };
        let this = &*self;
        let f = move || f(this);
        this.finish_buffer_swap_with(swap, f);
    }

    pub unsafe fn try_start_buffer_swap(&mut self) -> Result<Swap<Capture<I>>, CaptureError<I>> {
        let inner = &*self.inner;
        let capture = inner.strategy.try_capture_readers(&mut self.tag)?;

        // `which` only needs to syncronize with the readers, which only
        // load `which` (`Ordering::Acquire`), so a `Release` ordering
        // is sufficient. The `Writer` cannot race with itself.
        inner.which.fetch_xor(true, Ordering::Release);

        let capture = inner.strategy.finish_capture_readers(&mut self.tag, capture);
        Ok(Swap(capture))
    }

    pub unsafe fn start_buffer_swap(&mut self) -> Swap<Capture<I>> {
        self.try_start_buffer_swap().expect("Could not swap buffers")
    }

    pub fn is_swap_complete(&self, swap: &mut Swap<Capture<I>>) -> bool {
        self.inner.strategy.readers_have_exited(&mut swap.0)
    }

    pub fn finish_buffer_swap(&self, swap: Swap<Capture<I>>) {
        fn drop_in_place() {}
        self.finish_buffer_swap_with(swap, drop_in_place)
    }

    #[allow(clippy::toplevel_ref_arg)]
    pub fn finish_buffer_swap_with<F: FnMut()>(&self, swap: Swap<Capture<I>>, ref mut f: F) {
        #[cold]
        #[inline(never)]
        fn cold<S: Strategy>(f: &mut dyn FnMut(), strategy: &S, capture: &mut S::Capture) {
            f();
            strategy.pause(capture);
        }

        fn finish_swap_with<S: Strategy>(strategy: &S, tag: &S::WriterTag, mut swap: S::Capture, f: &mut dyn FnMut()) {
            let on_drop = FinishSwapOnDrop {
                strategy,
                swap: &mut swap,
            };

            while !strategy.readers_have_exited(on_drop.swap) {
                cold(f, strategy, on_drop.swap);
            }

            core::mem::forget(on_drop);

            strategy.finish_capture(tag, swap);
        }

        finish_swap_with(&self.inner.strategy, &self.tag, swap.0, f)
    }
}

impl<I: StrongBuffer> AsRef<Buffer<I>> for Writer<I> {
    fn as_ref(&self) -> &Buffer<I> { self.get() }
}

impl<I: StrongBuffer> AsMut<Buffer<I>> for Writer<I> {
    fn as_mut(&mut self) -> &mut Buffer<I> { self.get_mut() }
}
