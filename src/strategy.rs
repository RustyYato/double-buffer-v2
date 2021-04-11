pub mod atomic;
pub mod local;

#[cfg(feature = "alloc")]
pub mod sync;

#[cfg(feature = "std")]
pub mod park {
    pub use super::sync::park::*;
}
