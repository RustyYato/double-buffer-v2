use core::sync::atomic::{AtomicBool, AtomicUsize, Ordering};

use crate::traits::Strategy;

#[cfg(feature = "alloc")]
type Strong<B> = std::sync::Arc<crate::base::Inner<[B; 2], AtomicStrategy>>;
#[cfg(feature = "alloc")]
type Weak<B> = std::sync::Weak<crate::base::Inner<[B; 2], AtomicStrategy>>;

#[cfg(feature = "alloc")]
pub fn new<B: Default>() -> (crate::base::Writer<Strong<B>>, crate::base::Reader<Weak<B>>) {
    from_buffers(B::default(), B::default())
}

#[cfg(feature = "alloc")]
pub fn from_buffers<B>(front: B, back: B) -> (crate::base::Writer<Strong<B>>, crate::base::Reader<Weak<B>>) {
    crate::base::new(std::sync::Arc::new(crate::base::Inner::from_raw_parts(
        AtomicStrategy::default(),
        [front, back],
    )))
}

#[derive(Default)]
pub struct AtomicStrategy {
    readers: AtomicUsize,
}

#[derive(Clone, Copy)]
pub struct ReaderTag(());
pub struct WriterTag(());
pub struct RawGuard(());

pub struct Capture(());
#[derive(Debug)]
pub struct CaptureError(());

unsafe impl Strategy for AtomicStrategy {
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
