use radium::Radium;

use core::{marker::PhantomData, mem::ManuallyDrop, ops::Deref, sync::atomic::Ordering};

use crate::traits::{Buffer, ReaderTagW, Strategy, StrongBuffer, WeakBuffer};

impl<I> Default for Reader<I>
where
    I: WeakBuffer + Default,
    I::Strategy: Default,
{
    fn default() -> Self {
        let strategy = <I::Strategy>::default();
        let tag = unsafe { strategy.reader_tag() };
        Self {
            tag,
            inner: I::default(),
        }
    }
}

pub struct Reader<I, T = ReaderTagW<I>> {
    tag: T,
    inner: I,
}

pub struct ReaderGuard<'reader, I: StrongBuffer, T: ?Sized = Buffer<I>> {
    value: &'reader T,
    raw: RawGuard<'reader, I>,
}

struct RawGuard<'reader, I: StrongBuffer> {
    reader: PhantomData<&'reader ()>,
    raw: ManuallyDrop<crate::traits::RawGuard<I>>,
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

    #[inline]
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

impl<I: WeakBuffer> Clone for Reader<I> {
    fn clone(&self) -> Self {
        let tag = match self.inner.upgrade() {
            Ok(inner) => unsafe { inner.strategy.reader_tag() },
            Err(_) => unsafe { <I::Strategy>::dangling_reader_tag() },
        };

        Reader {
            inner: self.inner.clone(),
            tag,
        }
    }
}

impl<I: Copy + WeakBuffer> Copy for Reader<I> where ReaderTagW<I>: Copy {}

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
