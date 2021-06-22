use crate::{thin::Thin, traits::Strategy};
use core::cell::{Cell, UnsafeCell};

#[cfg(feature = "alloc")]
type Strong<B> = std::sync::Arc<crate::base::Inner<[B; 2], LocalSavingStrategy>>;
#[cfg(feature = "alloc")]
type Weak<B> = std::sync::Weak<crate::base::Inner<[B; 2], LocalSavingStrategy>>;

#[cfg(feature = "alloc")]
pub fn new<B: Default>() -> (crate::base::Writer<Strong<B>>, crate::base::Reader<Weak<B>>) {
    from_buffers(B::default(), B::default())
}

#[cfg(feature = "alloc")]
pub fn from_buffers<B>(front: B, back: B) -> (crate::base::Writer<Strong<B>>, crate::base::Reader<Weak<B>>) {
    crate::base::new(std::sync::Arc::new(crate::base::Inner::from_raw_parts(
        LocalSavingStrategy::default(),
        [front, back],
    )))
}

#[derive(Default)]
pub struct LocalSavingStrategy {
    tag_list: UnsafeCell<Vec<Thin<Cell<usize>>>>,
}

type Id = Thin<Cell<usize>>;

pub struct RawGuard {
    tag: Id,
}

pub struct FastCapture(());

pub struct Capture {
    active: Vec<(usize, Id)>,
}

pub struct ReaderTag(Id);
pub struct WriterTag(());

unsafe impl Strategy for LocalSavingStrategy {
    type Which = Cell<bool>;
    type ReaderTag = ReaderTag;
    type WriterTag = WriterTag;
    type RawGuard = RawGuard;

    type FastCapture = FastCapture;
    type CaptureError = ();
    type Capture = Capture;

    #[inline]
    unsafe fn reader_tag(&self) -> Self::ReaderTag {
        let tag = Thin::new(Cell::new(0));
        let list = &mut *self.tag_list.get();
        list.push(tag.clone());
        ReaderTag(tag)
    }

    #[inline]
    unsafe fn writer_tag(&self) -> Self::WriterTag { WriterTag(()) }

    #[inline]
    fn try_capture_readers(&self, _: &mut Self::WriterTag) -> Result<Self::FastCapture, Self::CaptureError> {
        Ok(FastCapture(()))
    }

    fn finish_capture_readers(&self, _: &mut Self::WriterTag, FastCapture(()): Self::FastCapture) -> Self::Capture {
        let list = unsafe { &mut *self.tag_list.get() };

        let mut active = Vec::with_capacity(list.len().min(8));

        let mut index = 0;
        // get rid of any dead readers and keep track of any active readers
        list.retain(|tag| {
            let is_alive = Thin::strong_count(tag) != 0;

            let value = tag.get();

            // if the reader is alive and reading the value
            if is_alive && value & 1 == 1 {
                active.push((value, tag.clone()))
            }

            index += 1;

            is_alive
        });

        Capture { active }
    }

    #[inline]
    fn readers_have_exited(&self, capture: &mut Self::Capture) -> bool {
        capture.active.retain(|(old_value, tag)| *old_value != tag.get());

        capture.active.is_empty()
    }

    #[inline]
    fn begin_guard(&self, reader_tag: &mut Self::ReaderTag) -> Self::RawGuard {
        #[cold]
        #[inline(never)]
        fn begin_guard_fail() -> ! {
            panic!("Previous reader guard was leaked");
        }
        let tag = &*reader_tag.0;

        if tag.get() & 1 == 0 {
            tag.set(tag.get().wrapping_add(1));
            RawGuard {
                tag: reader_tag.0.clone(),
            }
        } else {
            begin_guard_fail()
        }
    }

    #[inline]
    unsafe fn end_guard(&self, guard: Self::RawGuard) {
        let tag = &*guard.tag;
        let value = tag.get();
        tag.set(value.wrapping_add(1));
    }
}
