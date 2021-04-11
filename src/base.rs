use crate::traits::{RawDoubleBuffer, RawParts, Strategy};

mod raw_buffers;
mod reader;
mod writer;

use raw_buffers::RawBuffers;

use radium::Radium;
pub use reader::Reader;
pub use writer::{Buffer, Capture, CaptureError, Split, SplitMut, Swap, Writer};

pub struct Inner<S, R: ?Sized, W = <S as Strategy>::Which> {
    which: W,
    pub strategy: S,
    raw: RawBuffers<R>,
}

pub fn new<I: RawParts>(inner: I) -> (Writer<I::Strong>, Reader<I::Weak>) {
    let (writer, reader) = inner.raw_parts();
    let strategy = &writer.strategy;
    unsafe {
        let writer_tag = strategy.writer_tag();
        let reader_tag = strategy.reader_tag();
        (
            Writer::from_raw_parts(writer, writer_tag),
            Reader::from_raw_parts(reader, reader_tag),
        )
    }
}

impl<S: Strategy, B> Inner<S, [B; 2]> {
    pub fn new(strategy: S, front: B, back: B) -> Self { Self::from_raw_parts(strategy, [front, back]) }
}

impl<S: Strategy, R: RawDoubleBuffer> Inner<S, R> {
    pub fn from_raw_parts(strategy: S, buffers: R) -> Self {
        Self {
            strategy,
            which: Radium::new(false),
            raw: RawBuffers::new(buffers),
        }
    }
}
