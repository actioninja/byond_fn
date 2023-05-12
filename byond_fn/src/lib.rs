//! A macro crate for defining functions callable from BYOND easily and ergonomically
//!
//! ## Usage
//!
//! Basic usage is as simple as:
//! ```
//! use byond_fn::byond_fn;
//!
//! #[byond_fn]
//! pub fn add(arg1: u8, arg2: u8) -> u8 {
//!     arg1 + arg2
//! }
//! ```
//! This will generate a extern "C" function called `add` that can be called from BYOND:
//!
//! `call_ext("example_name.dll", "add")("2", "2") // returns 4`
//!
//!
//! ## Optional Parameters
//!
//! If a parameter is an `Option`, it will be optional to call from BYOND.
//!
//! All optional parameters must be at the end of the parameter list.
//!

pub use byond_fn_impl::*;

#[cfg(feature = "ffi_v2")]
pub mod ffi_v2;
pub mod str_ffi;

#[cfg(all(not(target_pointer_width = "32"), not(feature = "allow_other_arch")))]
compile_error!(
    r#"
You are attempting to compile this crate for a non-32-bit architecture.
Standard BYOND is 32-bit only, and requires a 32 bit target to properly link.
    - common 32-bit targets are `i686-pc-windows-msvc` and `i686-unknown-linux-gnu`
    - You likely need `cross` to compile for 32 bit on linux.
    - If you are sure you want to do this, you can enable the `allow_other_arch` feature
"#
);
