use core::cell::UnsafeCell;

use crate::traits::RawDoubleBuffer;

#[repr(transparent)]
pub(crate) struct RawBuffers<R: ?Sized>(UnsafeCell<R>);

unsafe impl<R: ?Sized + Send> Send for RawBuffers<R> {}
unsafe impl<R: ?Sized + Send + Sync> Sync for RawBuffers<R> {}

impl<R> RawBuffers<R> {
    pub const fn new(buffers: R) -> Self { Self(UnsafeCell::new(buffers)) }
}

impl<R: ?Sized + RawDoubleBuffer> RawBuffers<R> {
    pub(crate) unsafe fn read(&self, which: bool) -> *const R::Buffer { RawDoubleBuffer::split(self.0.get(), which).1 }

    pub(crate) unsafe fn split(&self, which: bool) -> (&R::Buffer, &R::Buffer) {
        let (writer, reader) = RawDoubleBuffer::split(self.0.get(), which);
        (&*writer, &*reader)
    }

    pub(crate) unsafe fn split_mut(&self, which: bool) -> (&mut R::Buffer, &R::Buffer) {
        let (writer, reader) = RawDoubleBuffer::split(self.0.get(), which);
        (&mut *writer, &*reader)
    }
}
