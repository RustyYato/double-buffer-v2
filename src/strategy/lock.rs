use core::cell::Cell;
use std::sync::atomic::{AtomicBool, Ordering};

use crate::traits::Strategy;
use parking_lot::{
    lock_api::{RawRwLock as _, RawRwLockFair, RawRwLockRecursive},
    RawRwLock,
};

pub struct LockStrategy {
    wait_to_swap: AtomicBool,
    lock: RawRwLock,
}

#[allow(clippy::declare_interior_mutable_const)]
impl LockStrategy {
    pub const INIT: Self = Self {
        lock: RawRwLock::INIT,
        wait_to_swap: AtomicBool::new(false),
    };
}

impl Default for LockStrategy {
    fn default() -> Self { Self::INIT }
}

pub struct ReaderTag(());
pub struct WriterTag(());
pub struct RawGuard(());

pub struct FastCapture(());
pub struct Capture(());

unsafe impl Strategy for LockStrategy {
    type Which = Cell<bool>;
    type ReaderTag = ReaderTag;
    type WriterTag = WriterTag;
    type RawGuard = RawGuard;

    type FastCapture = FastCapture;
    type Capture = Capture;
    type CaptureError = core::convert::Infallible;

    unsafe fn reader_tag(&self) -> Self::ReaderTag { ReaderTag(()) }

    unsafe fn writer_tag(&self) -> Self::WriterTag { WriterTag(()) }

    fn try_capture_readers(&self, _: &mut Self::WriterTag) -> Result<Self::FastCapture, Self::CaptureError> {
        Ok(FastCapture(()))
    }

    fn finish_capture_readers(&self, _: &mut Self::WriterTag, _: Self::FastCapture) -> Self::Capture {
        self.wait_to_swap.store(true, Ordering::Release);
        self.lock.lock_exclusive();
        unsafe {
            self.lock.unlock_exclusive();
        }
        self.wait_to_swap.store(false, Ordering::Release);
        Capture(())
    }

    fn readers_have_exited(&self, _: &mut Self::Capture) -> bool { true }

    fn begin_guard(&self, _: &mut Self::ReaderTag) -> Self::RawGuard {
        self.lock.lock_shared_recursive();
        RawGuard(())
    }

    unsafe fn end_guard(&self, _: Self::RawGuard) {
        if self.wait_to_swap.load(Ordering::Acquire) {
            self.lock.unlock_shared_fair();
        } else {
            self.lock.unlock_shared();
        }
    }
}
