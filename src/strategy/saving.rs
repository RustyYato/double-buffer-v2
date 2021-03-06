use crate::{athin::Athin, traits::Strategy};
use core::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
#[cfg(feature = "std")]
use parking_lot_core::SpinWait;
use std::vec::Vec;

#[cfg(feature = "std")]
use parking_lot::Mutex;
#[cfg(not(feature = "std"))]
use spin::Mutex;

#[cfg(feature = "std")]
pub(crate) mod park;

#[cfg(feature = "alloc")]
type Strong<B> = std::sync::Arc<crate::base::Inner<[B; 2], SavingStrategy>>;
#[cfg(feature = "alloc")]
type Weak<B> = std::sync::Weak<crate::base::Inner<[B; 2], SavingStrategy>>;

#[cfg(feature = "alloc")]
pub fn new<B: Default>() -> (crate::base::Writer<Strong<B>>, crate::base::Reader<Weak<B>>) {
    from_buffers(B::default(), B::default())
}

#[cfg(feature = "alloc")]
pub fn from_buffers<B>(front: B, back: B) -> (crate::base::Writer<Strong<B>>, crate::base::Reader<Weak<B>>) {
    crate::base::new(std::sync::Arc::new(crate::base::Inner::from_raw_parts(
        SavingStrategy::default(),
        [front, back],
    )))
}

#[derive(Default)]
pub struct SavingStrategy {
    tag_list: Mutex<Vec<Athin<AtomicUsize>>>,
}

pub struct RawGuard {
    tag: Athin<AtomicUsize>,
}

pub struct FastCapture(());

pub struct Capture {
    active: Vec<(usize, Athin<AtomicUsize>)>,
    #[cfg(feature = "std")]
    backoff: SpinWait,
}

pub struct ReaderTag(Athin<AtomicUsize>);
pub struct WriterTag(());

unsafe impl Strategy for SavingStrategy {
    type Which = AtomicBool;
    type ReaderTag = ReaderTag;
    type WriterTag = WriterTag;
    type RawGuard = RawGuard;

    type FastCapture = FastCapture;
    type CaptureError = core::convert::Infallible;
    type Capture = Capture;

    unsafe fn dangling_reader_tag() -> Self::ReaderTag { ReaderTag(Athin::dangling()) }

    #[inline]
    unsafe fn reader_tag(&self) -> Self::ReaderTag {
        let tag = Athin::new(AtomicUsize::new(0));
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
        let mut list = self.tag_list.lock();

        let mut active = Vec::with_capacity(list.len().min(8));

        // get rid of any dead readers and keep track of any active readers
        list.retain(|tag| {
            let is_alive = Athin::strong_count(tag) != 0;

            let value = tag.load(Ordering::Acquire);

            // if the reader is alive and reading the value
            if is_alive && value & 1 == 1 {
                active.push((value, tag.clone()))
            }

            is_alive
        });

        Capture {
            active,
            #[cfg(feature = "std")]
            backoff: SpinWait::new(),
        }
    }

    #[inline]
    fn readers_have_exited(&self, capture: &mut Self::Capture) -> bool {
        capture
            .active
            .retain(|(old_value, tag)| *old_value == tag.load(Ordering::Relaxed));

        let readers_have_exited = capture.active.is_empty();

        if readers_have_exited {
            core::sync::atomic::fence(Ordering::SeqCst);
        }

        readers_have_exited
    }

    #[cold]
    #[cfg(feature = "std")]
    fn pause(&self, capture: &mut Self::Capture) {
        if !capture.backoff.spin() {
            capture.backoff.reset()
        }
    }

    #[inline]
    fn begin_guard(&self, tag: &mut Self::ReaderTag) -> Self::RawGuard {
        #[cold]
        #[inline(never)]
        fn begin_guard_fail() -> ! {
            panic!("Previous reader guard was leaked");
        }
        if tag.0.fetch_add(1, Ordering::Acquire) & 1 == 0 {
            RawGuard { tag: tag.0.clone() }
        } else {
            begin_guard_fail()
        }
    }

    #[inline]
    unsafe fn end_guard(&self, guard: Self::RawGuard) { guard.tag.fetch_add(1, Ordering::Release); }
}
