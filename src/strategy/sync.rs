use core::sync::atomic::{AtomicBool, AtomicUsize, Ordering};

use crate::traits::Strategy;
use parking_lot_core::{park, unpark_all, unpark_filter, FilterOp, ParkToken, SpinWait, UnparkToken};

const SWAP_TOKEN: ParkToken = ParkToken(0);
const READ_TOKEN: ParkToken = ParkToken(1);
const SWAP_HANDOFF: UnparkToken = UnparkToken(0);
const UNPARK_ALL: UnparkToken = UnparkToken(1);

const ONE_READER: usize = 0b100;
const PENDING_SWAP: usize = 0b01;
const PENDING_READERS: usize = 0b10;

pub struct SyncStrategy {
    lock: AtomicUsize,
}

fn timeout() -> std::time::Instant { std::time::Instant::now() + std::time::Duration::from_micros(100) }

#[allow(clippy::declare_interior_mutable_const)]
impl SyncStrategy {
    pub const INIT: Self = Self {
        lock: AtomicUsize::new(0),
    };
}

impl Default for SyncStrategy {
    fn default() -> Self { Self::INIT }
}

pub struct ReaderTag(());
pub struct WriterTag(());
pub struct RawGuard(());

pub struct Capture(());

unsafe impl Strategy for SyncStrategy {
    type Which = AtomicBool;
    type ReaderTag = ReaderTag;
    type WriterTag = WriterTag;
    type RawGuard = RawGuard;

    type FastCapture = Capture;
    type Capture = Capture;
    type CaptureError = core::convert::Infallible;

    unsafe fn reader_tag(&self) -> Self::ReaderTag { ReaderTag(()) }

    unsafe fn writer_tag(&self) -> Self::WriterTag { WriterTag(()) }

    fn try_capture_readers(&self, _: &mut Self::WriterTag) -> Result<Self::FastCapture, Self::CaptureError> {
        Ok(Capture(()))
    }

    fn finish_capture_readers(&self, _: &mut Self::WriterTag, _: Self::FastCapture) -> Self::Capture { Capture(()) }

    fn readers_have_exited(&self, _: &mut Self::Capture) -> bool { self.lock.load(Ordering::Acquire) == 0 }

    fn finish_capture(&self, _: &Self::WriterTag, _: Self::Capture) {
        let lock = self.lock.fetch_and(!PENDING_READERS, Ordering::AcqRel);

        if lock != 0 {
            self.finish_capture_slow()
        }
    }

    fn pause(&self, _: &mut Self::Capture) {
        self.lock.fetch_or(PENDING_SWAP, Ordering::Release);

        let key = self as *const _ as usize;
        let validate = || self.lock.load(Ordering::Acquire) & !PENDING_SWAP != 0;
        let before_sleep = || {};
        let timed_out = |_, _| {};

        unsafe {
            park(key, validate, before_sleep, timed_out, SWAP_TOKEN, Some(timeout()));
        }

        self.lock.fetch_and(!PENDING_SWAP, Ordering::Release);
    }

    fn begin_guard(&self, _: &mut Self::ReaderTag) -> Self::RawGuard {
        let should_wait = self
            .lock
            .fetch_update(Ordering::AcqRel, Ordering::Acquire, |lock| {
                if lock & PENDING_SWAP != 0 {
                    return None
                }

                lock.checked_add(ONE_READER)
            })
            .is_err();

        if should_wait {
            self.begin_guard_slow()
        }

        RawGuard(())
    }

    unsafe fn end_guard(&self, _: Self::RawGuard) {
        // if last reader and there is a pending swap
        let pending_swap = self.lock.fetch_sub(ONE_READER, Ordering::Release) != PENDING_SWAP | ONE_READER;
        if pending_swap {
            self.end_guard_slow()
        }
    }
}

impl SyncStrategy {
    #[cold]
    #[inline(never)]
    fn finish_capture_slow(&self) {
        unsafe {
            let key = self as *const _ as usize;
            unpark_all(key, UNPARK_ALL);
        }
    }

    #[cold]
    #[inline(never)]
    fn begin_guard_slow(&self) {
        let mut lock = self.lock.load(Ordering::Acquire);
        let mut spin = SpinWait::new();

        loop {
            if lock & PENDING_SWAP == 0 {
                if let Some(next_lock) = lock.checked_add(ONE_READER) {
                    if let Err(l) =
                        self.lock
                            .compare_exchange_weak(lock, next_lock, Ordering::AcqRel, Ordering::Acquire)
                    {
                        lock = l;
                        if l & PENDING_SWAP != 0 {
                            core::hint::spin_loop();
                            continue
                        }
                    } else {
                        return
                    }
                }
            }

            if spin.spin() {
                lock = self.lock.load(Ordering::Acquire);
                continue
            }

            let key = self as *const _ as usize;
            let validate = || self.lock.load(Ordering::Acquire) & PENDING_SWAP != 0;
            let before_sleep = || {};
            let timed_out = |_, _| {};

            unsafe {
                park(key, validate, before_sleep, timed_out, READ_TOKEN, Some(timeout()));
            }

            spin.reset();
            lock = self.lock.load(Ordering::Acquire);
        }
    }

    #[cold]
    #[inline(never)]
    fn end_guard_slow(&self) {
        let key = self as *const _ as usize;
        let mut found_swap = false;
        let filter = |park_token| {
            if park_token == SWAP_TOKEN {
                found_swap = true;
                FilterOp::Unpark
            } else if found_swap {
                FilterOp::Stop
            } else {
                FilterOp::Skip
            }
        };
        let callback = |_| SWAP_HANDOFF;
        unsafe {
            unpark_filter(key, filter, callback);
        }
    }
}
