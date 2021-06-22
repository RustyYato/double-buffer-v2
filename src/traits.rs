use core::{cell::Cell, ops::Deref, sync::atomic::AtomicBool};

use crate::base::Inner;
use radium::Radium;

pub type Buffer<I> = <<I as StrongBuffer>::Raw as RawDoubleBuffer>::Buffer;
pub type Capture<I> = <<I as StrongBuffer>::Strategy as Strategy>::Capture;
pub type CaptureError<I> = <<I as StrongBuffer>::Strategy as Strategy>::CaptureError;
pub type WriterTag<I> = <<I as StrongBuffer>::Strategy as Strategy>::WriterTag;
pub type ReaderTag<I> = <<I as StrongBuffer>::Strategy as Strategy>::ReaderTag;

pub unsafe trait RawParts {
    type Strategy: Strategy;
    type Raw: RawDoubleBuffer + ?Sized;

    type Strong: StrongBuffer<Weak = Self::Weak, Strategy = Self::Strategy, Raw = Self::Raw>;
    type Weak: WeakBuffer<Strong = Self::Strong, Strategy = Self::Strategy, Raw = Self::Raw>;

    fn raw_parts(self) -> (Self::Strong, Self::Weak);
}

pub unsafe trait StrongBuffer: Clone + Deref<Target = Inner<Self::Raw, Self::Strategy>> {
    type Strategy: Strategy;
    type Raw: RawDoubleBuffer + ?Sized;

    type Weak: WeakBuffer<Strong = Self, Strategy = Self::Strategy, Raw = Self::Raw>;

    fn downgrade(&self) -> Self::Weak;
}

pub unsafe trait WeakBuffer: Clone {
    type Strategy: Strategy;
    type Raw: RawDoubleBuffer + ?Sized;

    type Strong: StrongBuffer<Weak = Self, Strategy = Self::Strategy, Raw = Self::Raw>;

    type UpgradeError: core::fmt::Debug;

    fn is_dangling(&self) -> bool;

    fn upgrade(&self) -> Result<Self::Strong, Self::UpgradeError>;
}

pub unsafe trait Strategy {
    type Which: TrustedRadium<Item = bool>;
    type ReaderTag;
    type WriterTag;
    type RawGuard;

    type FastCapture;
    type CaptureError: core::fmt::Debug;
    type Capture;

    unsafe fn reader_tag(&self) -> Self::ReaderTag;

    unsafe fn writer_tag(&self) -> Self::WriterTag;

    fn try_capture_readers(&self, tag: &mut Self::WriterTag) -> Result<Self::FastCapture, Self::CaptureError>;

    fn finish_capture_readers(&self, _: &mut Self::WriterTag, capture: Self::FastCapture) -> Self::Capture;

    fn readers_have_exited(&self, capture: &mut Self::Capture) -> bool;

    fn finish_capture(&self, _: &Self::WriterTag, _capture: Self::Capture) {}

    #[inline]
    fn pause(&self, _: &mut Self::Capture) {}

    fn begin_guard(&self, tag: &mut Self::ReaderTag) -> Self::RawGuard;

    unsafe fn end_guard(&self, guard: Self::RawGuard);
}

pub unsafe trait RawDoubleBuffer {
    type Buffer: ?Sized;

    unsafe fn split(this: *mut Self, which: bool) -> (*mut Self::Buffer, *const Self::Buffer);
}

pub unsafe trait TrustedRadium: Radium {
    #[doc(hidden)]
    unsafe fn load_unsync(&self) -> Self::Item;
}

unsafe impl<T: Copy> TrustedRadium for Cell<T>
where
    Self: Radium<Item = T>,
{
    unsafe fn load_unsync(&self) -> Self::Item { self.get() }
}

unsafe impl TrustedRadium for AtomicBool {
    unsafe fn load_unsync(&self) -> Self::Item { core::ptr::read(self as *const Self as *const bool) }
}

pub trait Operation<B: ?Sized>: Sized {
    fn apply(&mut self, buffer: &mut B);
    fn apply_final(mut self, buffer: &mut B) { self.apply(buffer) }
}
