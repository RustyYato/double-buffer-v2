use crate::{thin::Thin, traits::Strategy};
use core::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use crossbeam_utils::Backoff;
use smallvec::SmallVec;

#[cfg(feature = "std")]
use parking_lot::Mutex;
#[cfg(not(feature = "std"))]
use spin::Mutex;

#[cfg(feature = "std")]
pub(crate) mod park;

#[derive(Default)]
pub struct SyncStrategy {
    tag_list: Mutex<SmallVec<[Thin<AtomicUsize>; 8]>>,
}

pub struct RawGuard {
    tag: Thin<AtomicUsize>,
}

pub struct FastCapture(());

pub struct Capture {
    active: SmallVec<[Thin<AtomicUsize>; 8]>,
    backoff: Backoff,
}

pub struct ReaderTag(Thin<AtomicUsize>);
pub struct WriterTag(());

unsafe impl Strategy for SyncStrategy {
    type Which = AtomicBool;
    type ReaderTag = ReaderTag;
    type WriterTag = WriterTag;
    type RawGuard = RawGuard;

    type FastCapture = FastCapture;
    type CaptureError = core::convert::Infallible;
    type Capture = Capture;

    #[inline]
    unsafe fn reader_tag(&self) -> Self::ReaderTag {
        let tag = Thin::new(AtomicUsize::new(0));
        self.tag_list.lock().push(tag.clone());
        ReaderTag(tag)
    }

    #[inline]
    unsafe fn writer_tag(&self) -> Self::WriterTag { WriterTag(()) }

    #[inline]
    fn try_capture_readers(&self, _: &mut Self::WriterTag) -> Result<Self::FastCapture, Self::CaptureError> {
        core::sync::atomic::fence(Ordering::SeqCst);
        Ok(FastCapture(()))
    }

    fn finish_capture_readers(&self, _: &mut Self::WriterTag, FastCapture(()): Self::FastCapture) -> Self::Capture {
        let mut active = SmallVec::new();

        self.tag_list.lock().retain(|tag| {
            let is_alive = Thin::strong_count(tag) != 1;

            if is_alive && tag.load(Ordering::Acquire) & 1 == 1 {
                active.push(tag.clone())
            }

            is_alive
        });

        Capture {
            active,
            backoff: Backoff::new(),
        }
    }

    #[inline]
    fn readers_have_exited(&self, capture: &mut Self::Capture) -> bool {
        capture.active.retain(|tag| tag.load(Ordering::Relaxed) & 1 == 1);

        let readers_have_exited = capture.active.is_empty();

        if readers_have_exited {
            core::sync::atomic::fence(Ordering::SeqCst);
        }

        readers_have_exited
    }

    #[cold]
    fn pause(&self, capture: &mut Self::Capture) { capture.backoff.snooze(); }

    #[inline]
    fn begin_guard(&self, tag: &mut Self::ReaderTag) -> Self::RawGuard {
        tag.0.fetch_add(1, Ordering::Acquire);
        RawGuard { tag: tag.0.clone() }
    }

    #[inline]
    unsafe fn end_guard(&self, guard: Self::RawGuard) { guard.tag.fetch_add(1, Ordering::Acquire); }
}
