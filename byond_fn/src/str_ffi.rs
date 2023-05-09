use serde::de::DeserializeOwned;
use serde::Serialize;
use std::borrow::Cow;
use std::cell::RefCell;
use std::ffi::{c_char, c_int, CStr, CString};
use std::slice;

// BYOND doesn't like receiving back an empty string, so throw back just a null byte instead.
const EMPTY_STRING: c_char = 0;
thread_local! {
    // to return a string, we need to store it somewhere that won't be dropped.
    // since BYOND doesn't care to free the memory we allocate, we can just reuse the same
    // allocation over and over.
    static RETURN_STRING: RefCell<CString> = RefCell::new(CString::default());
}

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

pub fn byond_return(value: impl StrReturn) -> *const c_char {
    let value = value.to_return();
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

pub trait StrReturn {
    fn to_return(self) -> Option<Vec<u8>>;
}

impl StrReturn for () {
    fn to_return(self) -> Option<Vec<u8>> {
        None
    }
}

impl StrReturn for &'static str {
    fn to_return(self) -> Option<Vec<u8>> {
        Some(self.as_bytes().to_vec())
    }
}

impl StrReturn for String {
    fn to_return(self) -> Option<Vec<u8>> {
        Some(self.into_bytes())
    }
}

impl StrReturn for Vec<u8> {
    fn to_return(self) -> Option<Vec<u8>> {
        Some(self)
    }
}

macro_rules! impl_str_return {
    ($($ty:ty),*) => {
        $(
            impl StrReturn for $ty {
                fn to_return(self) -> Option<Vec<u8>> {
                    Some(self.to_string().into_bytes())
                }
            }
        )*
    };
}

impl_str_return!(i8, i16, i32, i64, i128, isize, u8, u16, u32, u64, u128, usize, bool);

pub trait StrArg<'a> {
    fn from_arg(arg: Cow<'a, str>) -> Self;
}

impl<'a> StrArg<'a> for String {
    fn from_arg(arg: Cow<'a, str>) -> Self {
        arg.into_owned()
    }
}

impl<'a> StrArg<'a> for Cow<'a, str> {
    fn from_arg(arg: Cow<'a, str>) -> Self {
        arg
    }
}

macro_rules! impl_str_arg {
    ($($ty:ty),*) => {
        $(
            impl<'a> StrArg<'a> for $ty {
                fn from_arg(arg: Cow<'a, str>) -> Self {
                    arg.parse().unwrap_or_default()
                }
            }
        )*
    };
}

impl_str_arg!(i8, i16, i32, i64, i128, isize, u8, u16, u32, u64, u128, usize, bool);

#[repr(transparent)]
#[derive(Debug)]
pub struct Json<T: Serialize + DeserializeOwned>(pub T);

impl<T: Serialize + DeserializeOwned> Json<T> {
    pub fn into_inner(self) -> T {
        self.0
    }
}

impl<T: Serialize + DeserializeOwned> From<T> for Json<T> {
    fn from(t: T) -> Self {
        Json(t)
    }
}

impl<T> StrReturn for Json<T>
where
    T: Serialize + DeserializeOwned,
{
    fn to_return(self) -> Option<Vec<u8>> {
        serde_json::to_vec(&self.0).ok()
    }
}

impl<'a, T> StrArg<'a> for Json<T>
where
    T: Serialize + DeserializeOwned,
{
    fn from_arg(arg: Cow<'a, str>) -> Self {
        Json(serde_json::from_str(&arg).unwrap())
    }
}
