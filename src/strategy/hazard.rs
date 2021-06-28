mod queue;
use std::sync::atomic::{AtomicBool, AtomicU32, Ordering};

use queue::{Queue, QueueNode};

use crate::traits::Strategy;
#[cfg(feature = "std")]
use parking_lot::{lock_api::RawMutex as _, Condvar, Mutex};

#[cfg(not(feature = "std"))]
pub struct HazardStrategy<W> {
    count: AtomicU32,
    queue: Queue<u32, true>,
    waiter: W,
}

#[cfg(feature = "std")]
pub struct HazardStrategy<W = Waiter> {
    count: AtomicU32,
    queue: Queue<AtomicU32, true>,
    waiter: W,
}

pub struct Spinner;
#[cfg(feature = "std")]
pub struct Waiter {
    mx: Mutex<()>,
    cv: Condvar,
}

pub trait Pause {
    fn pause(&self);

    fn notify(&self);
}

impl Pause for Spinner {
    fn pause(&self) {}

    fn notify(&self) {}
}

#[cfg(feature = "std")]
impl Pause for Waiter {
    fn pause(&self) {
        self.cv
            .wait_for(&mut self.mx.lock(), std::time::Duration::from_micros(100));
    }

    fn notify(&self) { self.cv.notify_one(); }
}

#[cfg(feature = "std")]
impl Default for HazardStrategy {
    fn default() -> Self { Self::new() }
}

#[cfg(feature = "std")]
impl HazardStrategy {
    pub fn new() -> Self {
        Self::with_waiter(Waiter {
            mx: Mutex::const_new(parking_lot::RawMutex::INIT, ()),
            cv: Condvar::new(),
        })
    }
}

impl HazardStrategy<Spinner> {
    pub const fn spinner() -> Self { Self::with_waiter(Spinner) }
}

impl<W> HazardStrategy<W> {
    const fn with_waiter(waiter: W) -> Self {
        Self {
            count: AtomicU32::new(0),
            queue: Queue::new(),
            waiter,
        }
    }
}

pub struct ReaderTag(());
pub struct WriterTag(());
pub struct RawGuard(QueueNode<AtomicU32, true>);

pub struct FastCapture(());
pub struct Capture(u32);
#[derive(Debug)]
pub struct CaptureError(());

unsafe impl Strategy for HazardStrategy {
    type Which = AtomicBool;
    type ReaderTag = ReaderTag;
    type WriterTag = WriterTag;
    type RawGuard = RawGuard;
    type FastCapture = FastCapture;
    type CaptureError = std::convert::Infallible;
    type Capture = Capture;

    unsafe fn dangling_reader_tag() -> Self::ReaderTag { ReaderTag(()) }

    unsafe fn reader_tag(&self) -> Self::ReaderTag { ReaderTag(()) }

    unsafe fn writer_tag(&self) -> Self::WriterTag { WriterTag(()) }

    fn try_capture_readers(&self, _: &mut Self::WriterTag) -> Result<Self::FastCapture, Self::CaptureError> {
        Ok(FastCapture(()))
    }

    fn finish_capture_readers(&self, _: &mut Self::WriterTag, _: Self::FastCapture) -> Self::Capture {
        Capture(self.count.fetch_add(1, Ordering::Release))
    }

    fn readers_have_exited(&self, &mut Capture(count): &mut Self::Capture) -> bool {
        !self.queue.any(|c| c.load(Ordering::Acquire) == count)
    }

    fn begin_guard(&self, _: &mut Self::ReaderTag) -> Self::RawGuard {
        let node = self.queue.alloc(AtomicU32::new(0));
        loop {
            let count = self.count.load(Ordering::Acquire);
            node.store(count, Ordering::Release);

            // prevent reordering the store after the next load
            core::sync::atomic::fence(Ordering::AcqRel);

            let new_count = self.count.load(Ordering::Acquire);
            if count == new_count {
                break
            }
        }
        RawGuard(node)
    }

    unsafe fn end_guard(&self, guard: Self::RawGuard) {
        drop(guard);
        self.waiter.notify();
    }

    fn pause(&self, _: &mut Self::Capture) { self.waiter.pause() }
}
