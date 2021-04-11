#![allow(clippy::missing_safety_doc)]
#![cfg_attr(not(feature = "std"), no_std)]

#[cfg(all(feature = "alloc", not(feature = "std")))]
extern crate alloc as std;

pub mod base;
pub mod strategy;
pub mod traits;

#[cfg(feature = "alloc")]
pub mod thin;

mod imp;
#[cfg(feature = "alloc")]
mod imp_alloc;
mod raw;

#[cfg(feature = "alloc")]
pub use imp_alloc::UpgradeError;
