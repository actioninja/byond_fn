use std::borrow::Cow;
use std::cell::RefCell;
use std::error::Error;
use std::ffi::{c_char, c_int, CStr, CString};
use std::fmt::Display;
use std::slice;

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

pub const ERR_WRONG_ARG_COUNT: &str = "Wrong number of arguments passed to function";
pub const ERR_ARG_PARSE: &str = "Failed to parse argument";
pub const ERR_RETURN_STR: &str = "Failed to serialize return value";

#[cfg(feature = "json_transport")]
pub const ERR_RETURN_SERIALIZE: &str = "Failed to serialize return value";
#[cfg(feature = "json_transport")]
pub const ERR_ARG_DESERIALIZE: &str = "Failed to deserialize arg value";

/// Turns the `argc` and `argv` arguments into a Rust `Vec<Cow<str>>`.
///
/// This is used internally, but is exposed in case you want the same functionality.
///
/// # Safety
/// Derefs the `argv` pointer.
/// This is intended to be used with the `argv` pointer that comes from the FFI bridge, and is
/// expected to be a valid pointer to an array of `argc` pointers to null-terminated strings.
/// If this is not the case, this function will cause undefined behavior.
pub unsafe fn parse_str_args<'a>(argc: c_int, argv: *const *const c_char) -> Vec<Cow<'a, str>> {
    slice::from_raw_parts(argv, argc as usize)
        .iter()
        .map(|ptr| CStr::from_ptr(*ptr))
        .map(|cstr| cstr.to_string_lossy())
        .collect()
}

pub fn byond_return(value: impl StrReturn) -> Result<*const c_char, TransportError> {
    match value.to_return() {
        Ok(value) => match value {
            None => Ok(&EMPTY_STRING),
            Some(vec) if vec.is_empty() => Ok(&EMPTY_STRING),
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
                Ok(cell.borrow().as_ptr())
            }),
        },
        Err(err) => Err(err),
    }
}

#[derive(Debug, Copy, Clone)]
pub enum TransportError {
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

/// Represents a type that can be returned to BYOND via string transport
/// (i.e. `byond_return`).
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

pub trait StrArg<'a>
    where
        Self: Sized,
{
    fn from_arg(arg: Option<Cow<'a, str>>) -> Result<Self, TransportError>;
}

impl<'a> StrArg<'a> for String {
    fn from_arg(arg: Option<Cow<'a, str>>) -> Result<Self, TransportError> {
        arg.map(|arg| arg.into_owned())
            .ok_or(TransportError::WrongArgCount)
    }
}

impl<'a> StrArg<'a> for Cow<'a, str> {
    fn from_arg(arg: Option<Cow<'a, str>>) -> Result<Self, TransportError> {
        arg.ok_or(TransportError::WrongArgCount)
    }
}

macro_rules! impl_str_arg {
    ($($ty:ty),*) => {
        $(
            impl<'a> StrArg<'a> for $ty {
                fn from_arg(arg: Option<Cow<'a, str>>) -> Result<Self, TransportError> {
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
    fn from_arg(arg: Option<Cow<'a, str>>) -> Result<Self, TransportError> {
        if let Some(arg) = arg {
            T::from_arg(Some(arg)).map(Some)
        } else {
            Ok(None)
        }
    }
}

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
    fn from_arg(arg: Option<Cow<'a, str>>) -> Result<Self, TransportError> {
        let arg = arg.ok_or(TransportError::WrongArgCount)?;
        let deserialized: T =
            serde_json::from_str(&arg).map_err(|_| TransportError::ArgDeserialize)?;
        Ok(Json(deserialized))
    }
}
