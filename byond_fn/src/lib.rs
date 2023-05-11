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
//! ## Errors
//!
//! Sometimes something may cause the parsing of arguments to fail, or returns to fail to serialize,
//! or similar.
//!
//! This will result in an early return from the function with an error string being sent to BYOND.
//!
//! Possible errors with string transport are:
//!
//! - `@@BYOND_FFI_ERROR@@: Invalid number of arguments to function`
//! - `@@BYOND_FFI_ERROR@@: Invalid argument type`
//! - `@@BYOND_FFI_ERROR@@: Invalid return type`
//! - `@@BYOND_FFI_ERROR@@: Invalid argument value`
//! - `@@BYOND_FFI_ERROR@@: Invalid return value`
//!
//! Errors are always prefixed with `@@BYOND_FFI_ERROR@@`
//!
//! ## What's generated
//! When a function is defined with `#[byond_fn]`, a function with the same name is generated in a
//! private module with neccessary trappings for calling from BYOND.
//! This generated function will parse the arguments from BYOND, call the original function, and
//! return the result to BYOND.
//!
//! Example:
//! ```
//! use byond_fn::byond_fn;
//!
//! #[byond_fn]
//! pub fn add(arg1: u8, arg2: u8) -> u8 {
//!     arg1 + arg2
//! }
//! ```
//! will generate an adjacent module that looks like this:
//! ```
//! mod __byond_fn_add {
//!     // necessary signatures for calling from BYOND
//!     #[no_mangle]
//!     pub unsafe extern "C" fn add(
//!         argc: ::std::os::raw::c_int,
//!         argv: *const *const ::std::os::raw::c_char,
//!     ) -> *const ::std::os::raw::c_char {
//!         let min_args = 2i32;
//!         let max_args = 2i32;
//!         // ensure a valid number of arguments were passed, otherwise early return with error
//!         if argc < min_args || argc > max_args {
//!             return byond_fn::str_ffi::byond_return(
//!                 byond_fn::str_ffi::TransportError::WrongArgCount.to_string(),
//!             )
//!             .unwrap();
//!         }
//!         // turn the raw pointers into a Vec of `Cow<str>`
//!         let args = byond_fn::str_ffi::parse_str_args(argc, argv);
//!         // bind the first argument to a value, early return with error if it fails to parse
//!         let arg1: u8 = match byond_fn::str_ffi::StrArg::from_arg(
//!             args.get(0usize).map(|arg| arg.clone()),
//!         ) {
//!             Ok(arg) => arg,
//!             Err(err) => {
//!                 return byond_fn::str_ffi::byond_return(err.to_string()).unwrap();
//!             }
//!         };
//!         let arg2: u8 = match byond_fn::str_ffi::StrArg::from_arg(
//!             args.get(1usize).map(|arg| arg.clone()),
//!         ) {
//!             Ok(arg) => arg,
//!             Err(err) => {
//!                 return byond_fn::str_ffi::byond_return(err.to_string()).unwrap();
//!             }
//!         };
//!         // call the original function, turn the result into a string, and return it to BYOND
//!         byond_fn::str_ffi::byond_return(super::add(arg1, arg2))
//!             .unwrap_or_else(|err| {
//!                 byond_fn::str_ffi::byond_return(err.to_string()).unwrap()
//!         })
//! }
//! }
//! ```
//!
//! ## Optional Parameters
//!
//! If a parameter is an `Option`, it will be optional to call from BYOND.
//!
//! All optional parameters must be at the end of the parameter list.
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

pub use byond_fn_impl::*;

#[cfg(feature = "ffi_v2")]
pub mod ffi_v2;
pub mod str_ffi;

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
