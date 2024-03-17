// Enable the `no_std` attribute if `no_std` is enabled
#![cfg_attr(not(feature = "std"), no_std)]

#[cfg(feature = "std")]
pub mod software_pass;
#[cfg(feature = "std")]
pub use software_pass::*;
