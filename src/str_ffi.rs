//! ## String FFI
//!
//! The default transport for this crate is string transport. As of BYOND 514, this is the only avalable transport.
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

use std::borrow::Cow;
use std::cell::RefCell;
use std::error::Error;
use std::ffi::{c_char, c_int, CStr, CString};
use std::fmt::Display;
use std::slice;
use std::str::Utf8Error;

use serde::de::DeserializeOwned;
use serde::Serialize;

// BYOND doesn't like receiving back an empty string, so throw back just a null byte instead.
const EMPTY_STRING: c_char = 0;
thread_local! {
    // to return a string, we need to store it somewhere that won't be dropped.
    // since BYOND doesn't care to free the memory we allocate, we can just reuse the same
    // allocation over and over.
    static RETURN_STRING: RefCell<CString> = RefCell::new(CString::default());
}

/// If a string returned is prefixed with this, it indicates that an error occurred.
pub const ERR_HEADER: &str = "@@BYOND_FFI_ERROR@@";

pub const ERR_BAD_UTF8: &str = "Invalid UTF-8 string";
pub const ERR_WRONG_ARG_COUNT: &str = "Wrong number of arguments passed to function";
pub const ERR_ARG_PARSE: &str = "Failed to parse argument";
pub const ERR_RETURN_STR: &str = "Failed to serialize return value";

#[cfg(feature = "json_transport")]
pub const ERR_RETURN_SERIALIZE: &str = "Failed to serialize return value";
#[cfg(feature = "json_transport")]
pub const ERR_ARG_DESERIALIZE: &str = "Failed to deserialize arg value";

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
/// expected to be a valid pointer to an array of `argc` pointers to null-terminated strings.
/// If this is not the case, this function will cause undefined behavior.
pub unsafe fn parse_str_args<'a>(
    argc: c_int,
    argv: *const *const c_char,
) -> Result<Vec<&'a str>, TransportError> {
    slice::from_raw_parts(argv, argc as usize)
        .iter()
        .map(|ptr| CStr::from_ptr(*ptr))
        .map(CStr::to_str)
        .map(|res| res.map_err(TransportError::BadUTF8))
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

#[derive(Debug, Copy, Clone)]
pub enum TransportError {
    BadUTF8(Utf8Error),
    WrongArgCount,
    ArgParse,
    ReturnStr,
    #[cfg(feature = "json_transport")]
    ArgDeserialize,
    #[cfg(feature = "json_transport")]
    ReturnSerialize,
}

impl Display for TransportError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{ERR_HEADER}: ")?;
        match self {
            Self::BadUTF8(utf) => write!(f, "{ERR_BAD_UTF8}; {utf}"),
            Self::WrongArgCount => write!(f, "{ERR_WRONG_ARG_COUNT}"),
            Self::ArgParse => write!(f, "{ERR_ARG_PARSE}"),
            Self::ReturnStr => write!(f, "{ERR_RETURN_STR}"),
            #[cfg(feature = "json_transport")]
            Self::ArgDeserialize => write!(f, "{ERR_ARG_DESERIALIZE}"),
            #[cfg(feature = "json_transport")]
            Self::ReturnSerialize => write!(f, "{ERR_RETURN_SERIALIZE}"),
        }
    }
}

impl Error for TransportError {}

impl From<Utf8Error> for TransportError {
    fn from(err: Utf8Error) -> Self {
        Self::BadUTF8(err)
    }
}

impl From<TransportError> for String {
    fn from(err: TransportError) -> Self {
        err.to_string()
    }
}

/// Represents a type that can be returned to BYOND via string transport
pub trait StrReturn {
    /// Converts the type into a `Vec<u8>` that can be returned to BYOND.
    /// If `None` is returned, an empty string will be returned to BYOND.
    fn to_return(self) -> Result<Option<Vec<u8>>, TransportError>;
}

impl StrReturn for () {
    fn to_return(self) -> Result<Option<Vec<u8>>, TransportError> {
        Ok(None)
    }
}

impl StrReturn for &'static str {
    fn to_return(self) -> Result<Option<Vec<u8>>, TransportError> {
        Ok(Some(self.as_bytes().to_vec()))
    }
}

impl StrReturn for String {
    fn to_return(self) -> Result<Option<Vec<u8>>, TransportError> {
        Ok(Some(self.into_bytes()))
    }
}

impl StrReturn for Vec<u8> {
    fn to_return(self) -> Result<Option<Vec<u8>>, TransportError> {
        Ok(Some(self))
    }
}

impl StrReturn for TransportError {
    fn to_return(self) -> Result<Option<Vec<u8>>, TransportError> {
        Err(self)
    }
}

macro_rules! impl_str_return {
    ($($ty:ty),*) => {
        $(
            impl StrReturn for $ty {
                fn to_return(self) -> Result<Option<Vec<u8>>, TransportError> {
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
    /// If `None` is passed, `Err(TransportError::WrongArgCount)` should be returned.
    fn from_arg(arg: Option<&'a str>) -> Result<Self, TransportError>;
}

impl<'a> StrArg<'a> for String {
    fn from_arg(arg: Option<&'a str>) -> Result<Self, TransportError> {
        arg.map(str::to_string).ok_or(TransportError::WrongArgCount)
    }
}

impl<'a> StrArg<'a> for Cow<'a, str> {
    fn from_arg(arg: Option<&'a str>) -> Result<Self, TransportError> {
        arg.map(Into::into).ok_or(TransportError::WrongArgCount)
    }
}

macro_rules! impl_str_arg {
    ($($ty:ty),*) => {
        $(
            impl<'a> StrArg<'a> for $ty {
                fn from_arg(arg: Option<&'a str>) -> Result<Self, TransportError> {
                    arg
                        .ok_or(TransportError::WrongArgCount)
                        .and_then(|arg| arg.parse().map_err(|_| TransportError::ArgParse))
                }
            }
        )*
    };
}

impl_str_arg!(i8, i16, i32, i64, i128, isize, u8, u16, u32, u64, u128, usize, bool);

impl<'a, T: StrArg<'a>> StrArg<'a> for Option<T> {
    fn from_arg(arg: Option<&'a str>) -> Result<Self, TransportError> {
        if let Some(arg) = arg {
            T::from_arg(Some(arg)).map(Some)
        } else {
            Ok(None)
        }
    }
}

/// Wraps another type to represent it should be parsed as JSON, or returned as JSON.
///
/// When a type is wrapped in this, it will be parsed as JSON when passed as an argument:
/// ```
/// use byond_fn::byond_fn;
/// use byond_fn::str_ffi::Json;
///
/// #[derive(serde::Serialize, serde::Deserialize)]
/// pub struct ExampleStruct {
///     field1: u32,
///     field2: String,
/// }
///
/// #[byond_fn]
/// fn example_fn(json: Json<ExampleStruct>) {
///     let mut unwrapped = json.into_inner();
///     // this is now a regular ExampleStruct.
///     unwrapped.field1 += 1;
/// }
/// ```
///
/// It is `repr(transparent)` so usage of this type should be zero-cost.
#[repr(transparent)]
#[derive(Debug)]
#[cfg(feature = "json_transport")]
pub struct Json<T: Serialize + DeserializeOwned>(pub T);

#[cfg(feature = "json_transport")]
impl<T: Serialize + DeserializeOwned> Json<T> {
    pub fn into_inner(self) -> T {
        self.0
    }
}

#[cfg(feature = "json_transport")]
impl<T: Serialize + DeserializeOwned> From<T> for Json<T> {
    fn from(t: T) -> Self {
        Json(t)
    }
}

#[cfg(feature = "json_transport")]
impl<T> StrReturn for Json<T>
where
    T: Serialize + DeserializeOwned,
{
    fn to_return(self) -> Result<Option<Vec<u8>>, TransportError> {
        if let Ok(serialized) = serde_json::to_vec(&self.0) {
            Ok(Some(serialized))
        } else {
            Err(TransportError::ReturnSerialize)
        }
    }
}

#[cfg(feature = "json_transport")]
impl<'a, T> StrArg<'a> for Json<T>
where
    T: Serialize + DeserializeOwned,
{
    fn from_arg(arg: Option<&'a str>) -> Result<Self, TransportError> {
        let arg = arg.ok_or(TransportError::WrongArgCount)?;
        let deserialized: T =
            serde_json::from_str(arg).map_err(|_| TransportError::ArgDeserialize)?;
        Ok(Json(deserialized))
    }
}
