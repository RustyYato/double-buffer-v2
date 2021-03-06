use core::cell::Cell;

use crate::traits::Strategy;

#[cfg(feature = "alloc")]
type Strong<B> = std::rc::Rc<crate::base::Inner<[B; 2], LocalStrategy>>;
#[cfg(feature = "alloc")]
type Weak<B> = std::rc::Weak<crate::base::Inner<[B; 2], LocalStrategy>>;

#[cfg(feature = "alloc")]
pub fn new<B: Default>() -> (crate::base::Writer<Strong<B>>, crate::base::Reader<Weak<B>>) {
    from_buffers(B::default(), B::default())
}

#[cfg(feature = "alloc")]
pub fn from_buffers<B>(front: B, back: B) -> (crate::base::Writer<Strong<B>>, crate::base::Reader<Weak<B>>) {
    crate::base::new(std::rc::Rc::new(crate::base::Inner::from_raw_parts(
        LocalStrategy::default(),
        [front, back],
    )))
}

#[derive(Default)]
pub struct LocalStrategy {
    readers: Cell<usize>,
}

#[derive(Clone, Copy)]
pub struct ReaderTag(());
pub struct WriterTag(());
pub struct RawGuard(());

pub struct Capture(());
#[derive(Debug)]
pub struct CaptureError(());

unsafe impl Strategy for LocalStrategy {
    type Which = Cell<bool>;
    type ReaderTag = ReaderTag;
    type WriterTag = WriterTag;
    type RawGuard = RawGuard;

    type FastCapture = Capture;
    type Capture = Capture;
    type CaptureError = CaptureError;

    unsafe fn dangling_reader_tag() -> Self::ReaderTag { ReaderTag(()) }

    unsafe fn reader_tag(&self) -> Self::ReaderTag { ReaderTag(()) }

    unsafe fn writer_tag(&self) -> Self::WriterTag { WriterTag(()) }

    fn try_capture_readers(&self, _: &mut Self::WriterTag) -> Result<Self::FastCapture, Self::CaptureError> {
        if self.readers.get() == 0 {
            Ok(Capture(()))
        } else {
            Err(CaptureError(()))
        }
    }

    fn finish_capture_readers(&self, _: &mut Self::WriterTag, capture: Self::FastCapture) -> Self::Capture { capture }

    fn readers_have_exited(&self, _: &mut Self::Capture) -> bool { true }

    fn begin_guard(&self, _: &mut Self::ReaderTag) -> Self::RawGuard {
        let readers = self.readers.get();
        self.readers
            .set(readers.checked_add(1).expect("Tried to create too many reader guards"));
        RawGuard(())
    }

    unsafe fn end_guard(&self, _: Self::RawGuard) { self.readers.set(self.readers.get().wrapping_sub(1)); }
}
