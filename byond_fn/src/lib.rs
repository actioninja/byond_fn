//! A macro crate for defining functions callable from BYOND easily and ergonomically
//!
//! ## Usage
//!
//!
//! ## Optional Paremeters
//!
//! ## JSON Transport
//!
//! Because string only transport limits the capability of transporting data, this crate also
//! supports JSON transport.
//!
//! JSON Transport is enabled via a feature flag, `json_transport`. This feature flag is enabled by
//! default.
//!
//! To utilize JSON transport, parameters should use the `Json` wrapper type:
//!
//! ```
//! use byond_fn::byond_fn;
//!
//! #[byond_fn]
//! pub fn add(left: Json<usize>, right: Json<usize>) -> usize {
//!    left.0 + right.0
//! }
//! ```
//!
//! parameters that use the `Json` wrapper type will attempt to deserialize the parameter from JSON
//! into the provided type. Return types that use the `Json` wrapper type will be serialized into
//! JSON before being sent to BYOND.
//!
//! If this fails, the function will early return an error string to BYOND.
//!
//!
//! `Json` requires the serde `Serialize` and `Deserialize` traits to be implemented for the type.
//!

#[cfg(feature = "ffi_v2")]
pub mod ffi_v2;
pub mod str_ffi;

pub use byond_fn_impl::*;

#[cfg(test)]
mod test {
    #[test]
    fn expected_simple() {}
}

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
