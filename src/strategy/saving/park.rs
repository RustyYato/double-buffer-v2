use crate::{
    strategy::saving::{Capture as RawCapture, FastCapture as RawFastCapture},
    traits::Strategy,
};
use core::sync::atomic::AtomicBool;
use parking_lot::Condvar;

#[derive(Default)]
pub struct SavingParkStrategy<const THREAD_COUNT: usize> {
    raw: super::SavingStrategy<THREAD_COUNT>,
    cv: Condvar,
}

pub struct FastCapture(RawFastCapture);
pub struct Capture<const THREAD_COUNT: usize>(RawCapture<THREAD_COUNT>);

pub struct ReaderTag(super::ReaderTag);
pub struct WriterTag(super::WriterTag);
pub struct RawGuard(super::RawGuard);

impl<const THREAD_COUNT: usize> SavingParkStrategy<THREAD_COUNT> {
    #[cold]
    #[inline(never)]
    fn park(&self) {
        self.cv
            .wait_for(&mut self.raw.tag_list.lock(), std::time::Duration::from_micros(100));
    }
}

unsafe impl<const THREAD_COUNT: usize> Strategy for SavingParkStrategy<THREAD_COUNT> {
    type Which = AtomicBool;
    type ReaderTag = ReaderTag;
    type WriterTag = WriterTag;
    type RawGuard = RawGuard;

    type FastCapture = FastCapture;
    type CaptureError = core::convert::Infallible;
    type Capture = Capture<THREAD_COUNT>;

    #[inline]
    unsafe fn reader_tag(&self) -> Self::ReaderTag { ReaderTag(self.raw.reader_tag()) }

    #[inline]
    unsafe fn writer_tag(&self) -> Self::WriterTag { WriterTag(self.raw.writer_tag()) }

    #[inline]
    fn try_capture_readers(
        &self,
        WriterTag(tag): &mut Self::WriterTag,
    ) -> Result<Self::FastCapture, Self::CaptureError> {
        self.raw.try_capture_readers(tag).map(FastCapture)
    }

    #[inline]
    fn finish_capture_readers(
        &self,
        WriterTag(tag): &mut Self::WriterTag,
        FastCapture(capture): Self::FastCapture,
    ) -> Self::Capture {
        Capture(self.raw.finish_capture_readers(tag, capture))
    }

    #[inline]
    fn readers_have_exited(&self, Capture(capture): &mut Self::Capture) -> bool {
        self.raw.readers_have_exited(capture)
    }

    #[cold]
    fn pause(&self, Capture(capture): &mut Self::Capture) {
        if !capture.backoff.spin() {
            self.park();
        }
    }

    #[inline]
    fn begin_guard(&self, ReaderTag(tag): &mut Self::ReaderTag) -> Self::RawGuard {
        RawGuard(self.raw.begin_guard(tag))
    }

    #[inline]
    unsafe fn end_guard(&self, RawGuard(guard): Self::RawGuard) { self.raw.end_guard(guard) }
}
