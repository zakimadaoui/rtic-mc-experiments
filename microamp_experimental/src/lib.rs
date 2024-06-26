#![deny(rust_2018_compatibility)]
#![deny(rust_2018_idioms)]
#![doc = include_str!("../README.md")]
#![deny(missing_docs)]
#![no_std]

pub use microamp_macros::shared;

mod cfail;
#[doc(hidden)]
pub mod export;
