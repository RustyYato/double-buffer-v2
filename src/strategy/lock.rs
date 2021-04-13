use core::cell::Cell;

use crate::traits::Strategy;
use parking_lot::{lock_api::RawRwLock as _, RawRwLock};

pub struct LockStrategy {
    lock: RawRwLock,
}

#[allow(clippy::declare_interior_mutable_const)]
impl LockStrategy {
    pub const INIT: Self = Self { lock: RawRwLock::INIT };
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
        self.lock.lock_exclusive();
        unsafe {
            self.lock.unlock_exclusive();
        }
        Capture(())
    }

    fn readers_have_exited(&self, _: &mut Self::Capture) -> bool { true }

    fn begin_guard(&self, _: &mut Self::ReaderTag) -> Self::RawGuard {
        self.lock.lock_shared();
        RawGuard(())
    }

    unsafe fn end_guard(&self, _: Self::RawGuard) { self.lock.unlock_shared(); }
}
