use radium::Radium;

use core::{marker::PhantomData, mem::ManuallyDrop, ops::Deref, sync::atomic::Ordering};

use crate::traits::{RawDoubleBuffer, Strategy, StrongBuffer, WeakBuffer};

pub struct Reader<I, T = <<I as WeakBuffer>::Strategy as Strategy>::ReaderTag> {
    tag: T,
    inner: I,
}

pub struct ReaderGuard<'reader, I: StrongBuffer, T: ?Sized = <<I as StrongBuffer>::Raw as RawDoubleBuffer>::Buffer> {
    value: &'reader T,
    raw: RawGuard<'reader, I>,
}

pub struct RawGuard<'reader, I: StrongBuffer> {
    reader: PhantomData<&'reader ()>,
    raw: ManuallyDrop<<<I as StrongBuffer>::Strategy as Strategy>::RawGuard>,
    keep_alive: I,
}

impl<I: StrongBuffer> Drop for RawGuard<'_, I> {
    fn drop(&mut self) { unsafe { self.keep_alive.strategy.end_guard(ManuallyDrop::take(&mut self.raw)) } }
}

impl<I, T> Reader<I, T> {
    pub(crate) unsafe fn from_raw_parts(inner: I, tag: T) -> Self { Self { tag, inner } }
}

impl<I: WeakBuffer> Reader<I> {
    pub(crate) fn new(inner: &I::Strong) -> Self {
        Self {
            tag: unsafe { inner.strategy.reader_tag() },
            inner: inner.downgrade(),
        }
    }

    pub fn try_clone(&self) -> Result<Self, I::UpgradeError> {
        let inner = self.inner.upgrade()?;
        let tag = unsafe { inner.strategy.reader_tag() };
        Ok(Reader {
            inner: self.inner.clone(),
            tag,
        })
    }

    pub fn is_dangling(&self) -> bool { self.inner.is_dangling() }

    #[inline]
    pub fn get(&mut self) -> ReaderGuard<'_, I::Strong> {
        self.try_get().expect("Tried to reader from a dangling `Reader<B>`")
    }

    #[inline]
    pub fn try_get(&mut self) -> Result<ReaderGuard<'_, I::Strong>, I::UpgradeError> {
        let keep_alive = self.inner.upgrade()?;
        let inner = &*keep_alive;
        let guard = inner.strategy.begin_guard(&mut self.tag);

        let which = inner.which.load(Ordering::Acquire);
        let buffer = unsafe { inner.raw.read(which) };

        Ok(ReaderGuard {
            value: unsafe { &*buffer },
            raw: RawGuard {
                reader: PhantomData,
                raw: ManuallyDrop::new(guard),
                keep_alive,
            },
        })
    }
}

impl<I: WeakBuffer<UpgradeError = core::convert::Infallible>> Clone for Reader<I> {
    fn clone(&self) -> Self {
        match self.try_clone() {
            Ok(reader) => reader,
            Err(infallible) => match infallible {},
        }
    }
}

impl<I: StrongBuffer, T: ?Sized> Deref for ReaderGuard<'_, I, T> {
    type Target = T;

    fn deref(&self) -> &Self::Target { self.value }
}

impl<'reader, I: StrongBuffer, T> ReaderGuard<'reader, I, T> {
    pub fn strategy(this: &Self) -> &I::Strategy { &this.raw.keep_alive.strategy }

    pub fn map<F: FnOnce(&T) -> &U, U: ?Sized>(this: Self, f: F) -> ReaderGuard<'reader, I, U> {
        ReaderGuard {
            value: f(this.value),
            raw: this.raw,
        }
    }

    pub fn try_map<F: FnOnce(&T) -> Option<&U>, U: ?Sized>(
        this: Self,
        f: F,
    ) -> Result<ReaderGuard<'reader, I, U>, Self> {
        match f(this.value) {
            None => Err(this),
            Some(value) => Ok(ReaderGuard { value, raw: this.raw }),
        }
    }
}
