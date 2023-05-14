//! ## String FFI
//!
//! The default transport for this crate is string transport. As of BYOND 514, this is the only avalable transport.
//!
//! ## Errors
//!
//! Sometimes something may cause the parsing of arguments to fail, or returns to fail to serialize,
//! or similar.
//!
//! In these cases, the function will return an error string to BYOND. Error strings will always be structured like so:
//!
//! `@@ERR@@|<error class>|<error type>|<error message>`
//!
//! The error class is an easily machine readable string that describes the general category of error that occurred.
//! Possible classes are:
//! - `FFI` - An error occurred while parsing arguments, serializing return values, or the function being called
//! incorrectly
//! -`JSON` - An error occurred while parsing or serializing JSON arguments or return values
//! - `FN` - An error occurred within the function itself being called and was returned as an `Err`
//!
//! The error type is an easily machine readable string that describes the specific error that occurred
//!
//! Error type is omitted for `FN` errors, as this would require each consumer to define their own errors.
//!
//! ## JSON Transport
//!
//! parameters that use the `Json` wrapper type will attempt to deserialize the parameter from JSON
//! into the provided type. Return types that use the `Json` wrapper type will be serialized into
//! JSON before being sent to BYOND.
//!
//! If this fails, the function will early return an error string to BYOND.
//!
//! `Json` requires the serde `Serialize` and `Deserialize` traits to be implemented for the type
//!
//! See `[Json](crate::str_ffi::Json)` for more information.
//!
//! ## What's generated
//! When a function is defined with `#[byond_fn]`, a function with the same name is generated in a
//! private module with necessary trappings for calling from BYOND.
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
//!     #[no_mangle]
//!     pub unsafe extern "C" fn add(
//!         argc: ::std::os::raw::c_int,
//!         argv: *const *const ::std::os::raw::c_char,
//!     ) -> *const ::std::os::raw::c_char {
//!         if argc < 2i32 || argc > 2i32 {
//!             return byond_fn::str_ffi::byond_return(
//!                 byond_fn::str_ffi::TransportError::WrongArgCount,
//!              );
//!         }
//!         let args = match byond_fn::str_ffi::parse_str_args(argc, argv) {
//!              Ok(args) => args,
//!              Err(err) => {
//!                  return byond_fn::str_ffi::byond_return(err);
//!              }
//!          };
//!          let arg1: u8 = match byond_fn::str_ffi::StrArg::from_arg(
//!              args.get(0usize).map(|arg| *arg),
//!          ) {
//!              Ok(arg) => arg,
//!              Err(err) => {
//!                  return byond_fn::str_ffi::byond_return(err);
//!              }
//!          };
//!          let arg2: u8 = match byond_fn::str_ffi::StrArg::from_arg(
//!              args.get(1usize).map(|arg| *arg),
//!          ) {
//!              Ok(arg) => arg,
//!              Err(err) => {
//!                  return byond_fn::str_ffi::byond_return(err);
//!              }
//!          };
//!          byond_fn::str_ffi::byond_return(super::add(arg1, arg2))
//!     }
//! }
//! ```

#[cfg(feature = "json_transport")]
pub mod json;

use std::borrow::Cow;
use std::cell::RefCell;
use std::error::Error;
use std::ffi::{c_char, c_int, CStr, CString};
use std::fmt::{Display, Formatter};
use std::path::{Path, PathBuf};
use std::slice;
use std::str::Utf8Error;

use crate::str_ffi::json::JsonError;

// BYOND doesn't like receiving back an empty string, so throw back just a null byte instead.
const EMPTY_STRING: c_char = 0;
thread_local! {
    // to return a string, we need to store it somewhere that won't be dropped.
    // since BYOND doesn't care to free the memory we allocate, we can just reuse the same
    // allocation over and over.
    static RETURN_STRING: RefCell<CString> = RefCell::new(CString::default());
}

/// This module contains easily machine parsable errors keys
pub mod error_keys {
    /// All returned error strings are prefixed with this
    pub const HEADER: &str = "@@ERR@@";

    pub const CLASS_FFI: &str = "FFI";
    pub const CLASS_JSON: &str = "JSON";
    pub const CLASS_FN: &str = "FN";

    pub const FFI_TYPE_BAD_UTF8: &str = "BAD_UTF8";
    pub const FFI_TYPE_WRONG_ARG_COUNT: &str = "WRONG_ARG_COUNT";
    pub const FFI_TYPE_ARG_PARSE: &str = "ARG_PARSE";
    pub const FFI_TYPE_RETURN_STR: &str = "RETURN_STR";

    #[cfg(feature = "json_transport")]
    pub const JSON_TYPE_SERIALIZE: &str = "SERIALIZE";
    #[cfg(feature = "json_transport")]
    pub const JSON_TYPE_DESERIALIZE: &str = "DESERIALIZE";
}

/// Turns the `argc` and `argv` arguments into a Rust `Vec<&str>`.
///
/// This is used internally, but is exposed in case you want the same functionality.
///
/// # Errors
///
/// If any of the arguments are not valid UTF-8, this will return a `TransportError::BadUTF8`.
///
/// # Safety
/// Derefs the `argv` pointer.
/// This is intended to be used with the `argv` pointer that comes from the FFI bridge, and is
/// expected to be a valid pointer to an array of `argc` count pointers to null-terminated strings.
/// If this is not the case, this function will cause undefined behavior.
pub unsafe fn parse_str_args<'a>(
    argc: c_int,
    argv: *const *const c_char,
) -> Result<Vec<&'a str>, FFIError> {
    let cstr = unsafe {
        slice::from_raw_parts(argv, argc as usize)
            .iter()
            .map(|ptr| CStr::from_ptr(*ptr))
    };
    cstr.map(CStr::to_str)
        .map(|res| res.map_err(TransportError::BadUTF8).map_err(Into::into))
        .collect()
}

/// A function to prep a value for returning to BYOND.
///
/// Converts the value into a string, and then returns a pointer to the string. The string is allocated into a
/// thread-local buffer, so it will be overwritten on the next call.
///
/// This is used internally, but is exposed in case you want the same functionality.
pub fn byond_return(value: impl StrReturn) -> *const c_char {
    let value = match value.to_return() {
        Ok(inner) => inner.filter(|inner| !inner.is_empty()),
        Err(err) => Some(err.to_string().into_bytes()),
    };
    match value {
        None => &EMPTY_STRING,
        Some(vec) if vec.is_empty() => &EMPTY_STRING,
        Some(vec) => RETURN_STRING.with(|cell| {
            // Panicking over an FFI boundary is bad form, so if a NUL ends up
            // in the result, just truncate.
            let cstring = CString::new(vec).unwrap_or_else(|err| {
                let post = err.nul_position();
                let mut vec = err.into_vec();
                vec.truncate(post);
                CString::new(vec).unwrap_or_default()
            });
            cell.replace(cstring);
            cell.borrow().as_ptr()
        }),
    }
}

#[derive(Debug)]
pub enum FFIError {
    TransportError(TransportError),
    OtherError(Box<dyn Error>),
    #[cfg(feature = "json_transport")]
    JsonError(JsonError),
}

impl Display for FFIError {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "{};", error_keys::HEADER)?;
        match self {
            FFIError::TransportError(err) => write!(f, "{err}"),
            FFIError::OtherError(err) => write!(f, "{err}"),
            #[cfg(feature = "json_transport")]
            FFIError::JsonError(err) => write!(f, "{err}"),
        }
    }
}

impl From<TransportError> for FFIError {
    fn from(err: TransportError) -> Self {
        Self::TransportError(err)
    }
}

impl From<Box<dyn Error>> for FFIError {
    fn from(err: Box<dyn Error>) -> Self {
        Self::OtherError(err)
    }
}

#[derive(Debug)]
pub enum TransportError {
    BadUTF8(Utf8Error),
    WrongArgCount {
        expected_min: usize,
        expected_max: usize,
        got: usize,
    },
    ArgParse {
        arg_name: String,
        actual_content: String,
    },
    ReturnStr(String),
}

impl Display for TransportError {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "{};", error_keys::CLASS_FFI)?;
        match self {
            Self::BadUTF8(utf) => write!(f, "{}; {}", error_keys::FFI_TYPE_BAD_UTF8, utf),
            Self::WrongArgCount {
                expected_min,
                expected_max,
                got,
            } => {
                let range_str = match (expected_min, expected_max) {
                    (min, max) if min == max => format!("{min}"),
                    (min, max) => format!("{min}-{max}"),
                };
                write!(f, "Expected {range_str} args, got {got}")
            }
            Self::ArgParse {
                arg_name,
                actual_content,
            } => write!(
                f,
                "{};Failed to parse argument \"{}\" (content was \"{}\")",
                error_keys::FFI_TYPE_ARG_PARSE,
                arg_name,
                actual_content,
            ),
            Self::ReturnStr(failed_return) => write!(
                f,
                "{};Failed to serialize return value \"{}\"",
                error_keys::FFI_TYPE_RETURN_STR,
                failed_return,
            ),
        }
    }
}

impl Error for TransportError {}

impl From<Utf8Error> for TransportError {
    fn from(err: Utf8Error) -> Self {
        Self::BadUTF8(err)
    }
}

/// Represents a type that can be returned to BYOND via string transport
pub trait StrReturn {
    /// Converts the type into a `Vec<u8>` that can be returned to BYOND.
    /// If `None` is returned, an empty string will be returned to BYOND.
    fn to_return(self) -> Result<Option<Vec<u8>>, FFIError>;
}

impl StrReturn for () {
    fn to_return(self) -> Result<Option<Vec<u8>>, FFIError> {
        Ok(None)
    }
}

impl StrReturn for &'static str {
    fn to_return(self) -> Result<Option<Vec<u8>>, FFIError> {
        Ok(Some(self.as_bytes().to_vec()))
    }
}

impl StrReturn for String {
    fn to_return(self) -> Result<Option<Vec<u8>>, FFIError> {
        Ok(Some(self.into_bytes()))
    }
}

impl StrReturn for Vec<u8> {
    fn to_return(self) -> Result<Option<Vec<u8>>, FFIError> {
        Ok(Some(self))
    }
}

impl StrReturn for FFIError {
    fn to_return(self) -> Result<Option<Vec<u8>>, FFIError> {
        Err(self)
    }
}

impl<T, E> StrReturn for Result<T, E>
where
    T: StrReturn,
    E: Error + 'static,
{
    fn to_return(self) -> Result<Option<Vec<u8>>, FFIError> {
        match self {
            Ok(inner) => inner.to_return(),
            Err(err) => Err(FFIError::OtherError(Box::new(err))),
        }
    }
}

impl StrReturn for TransportError {
    fn to_return(self) -> Result<Option<Vec<u8>>, FFIError> {
        Err(FFIError::TransportError(self))
    }
}

macro_rules! impl_str_return {
    ($($ty:ty),*) => {
        $(
            impl StrReturn for $ty {
                fn to_return(self) -> Result<Option<Vec<u8>>, FFIError> {
                    Ok(Some(self.to_string().into_bytes()))
                }
            }
        )*
    };
}

impl_str_return!(i8, i16, i32, i64, i128, isize, u8, u16, u32, u64, u128, usize, bool);

/// Represents a type that can be parsed from BYOND via string transport
pub trait StrArg<'a>
where
    Self: Sized,
{
    /// Parses the type from a string slice.
    /// This function should *never* be called directly. Only through `map_arg`.
    fn from_arg(_arg: &'a str, _arg_name: &str) -> Result<Self, FFIError> {
        let type_name = std::any::type_name::<Self>();
        unimplemented!("from_arg not implemented for type \"{type_name}\" (This is a bug)")
    }

    /// Maps an argument to a type. Handles error cases.
    fn map_arg(
        arg: Option<&'a str>,
        expected_min: usize,
        expected_max: usize,
        arg_name: &str,
        arg_num: usize,
    ) -> Result<Self, FFIError> {
        if let Some(arg) = arg {
            Self::from_arg(arg, arg_name)
        } else {
            Err(FFIError::TransportError(TransportError::WrongArgCount {
                expected_min,
                expected_max,
                got: arg_num,
            }))
        }
    }
}

impl<'a> StrArg<'a> for String {
    fn from_arg(arg: &'a str, _arg_name: &str) -> Result<Self, FFIError> {
        Ok(arg.to_string())
    }
}

impl<'a> StrArg<'a> for Cow<'a, str> {
    fn from_arg(arg: &'a str, _arg_name: &str) -> Result<Self, FFIError> {
        Ok(arg.into())
    }
}

impl<'a> StrArg<'a> for &'a str {
    fn from_arg(arg: &'a str, _arg_name: &str) -> Result<Self, FFIError> {
        Ok(arg)
    }
}

impl<'a> StrArg<'a> for &'a Path {
    fn from_arg(arg: &'a str, _arg_name: &str) -> Result<Self, FFIError> {
        Ok(Path::new(arg))
    }
}

impl<'a> StrArg<'a> for PathBuf {
    fn from_arg(arg: &'a str, _arg_name: &str) -> Result<Self, FFIError> {
        Ok(PathBuf::from(arg))
    }
}

macro_rules! impl_str_arg {
    ($($ty:ty),*) => {
        $(
            impl<'a> StrArg<'a> for $ty {
                fn from_arg(arg: &'a str, arg_name: &str) -> Result<Self, FFIError> {
                    arg.parse().map_err(|_| FFIError::TransportError(TransportError::ArgParse {
                        arg_name: arg_name.to_string(),
                        actual_content: arg.to_string(),
                    }))
                }
            }
        )*
    };
}

impl_str_arg!(i8, i16, i32, i64, i128, isize, u8, u16, u32, u64, u128, usize, bool);

impl<'a, T: StrArg<'a>> StrArg<'a> for Option<T> {
    fn from_arg(arg: &'a str, arg_name: &str) -> Result<Self, FFIError> {
        T::from_arg(arg, arg_name).map(Some)
    }

    fn map_arg(
        arg: Option<&'a str>,
        _expected_min: usize,
        _expected_max: usize,
        arg_name: &str,
        _arg_num: usize,
    ) -> Result<Self, FFIError> {
        if let Some(arg) = arg {
            Self::from_arg(arg, arg_name)
        } else {
            Ok(None)
        }
    }
}
