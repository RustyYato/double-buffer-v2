use core::sync::atomic::{AtomicBool, AtomicUsize, Ordering};

use crate::traits::Strategy;

pub struct Atomic {
    readers: AtomicUsize,
}

pub struct ReaderTag(());
pub struct WriterTag(());
pub struct RawGuard(());

pub struct Capture(());
#[derive(Debug)]
pub struct CaptureError(());

unsafe impl Strategy for Atomic {
    type Which = AtomicBool;
    type ReaderTag = ReaderTag;
    type WriterTag = WriterTag;
    type RawGuard = RawGuard;

    type FastCapture = Capture;
    type Capture = Capture;
    type CaptureError = CaptureError;

    unsafe fn reader_tag(&self) -> Self::ReaderTag { ReaderTag(()) }

    unsafe fn writer_tag(&self) -> Self::WriterTag { WriterTag(()) }

    fn try_capture_readers(&self, _: &mut Self::WriterTag) -> Result<Self::FastCapture, Self::CaptureError> {
        if self.readers.load(Ordering::Acquire) == 0 {
            Ok(Capture(()))
        } else {
            Err(CaptureError(()))
        }
    }

    fn finish_capture_readers(&self, _: &mut Self::WriterTag, capture: Self::FastCapture) -> Self::Capture { capture }

    fn readers_have_exited(&self, _: &mut Self::Capture) -> bool { true }

    fn begin_guard(&self, _: &mut Self::ReaderTag) -> Self::RawGuard {
        self.readers
            .fetch_update(Ordering::AcqRel, Ordering::Relaxed, |readers| readers.checked_add(1))
            .expect("Tried to create too many reader guards");
        RawGuard(())
    }

    unsafe fn end_guard(&self, _: Self::RawGuard) { self.readers.fetch_sub(1, Ordering::Release); }
}
