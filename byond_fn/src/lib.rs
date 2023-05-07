#[cfg(feature = "ffi_v2")]
pub mod ffi_v2;

use std::borrow::Cow;
use std::cell::RefCell;
use std::ffi::{c_char, c_int, CStr, CString};
use std::slice;

pub use byond_fn_impl::*;

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

pub fn byond_return(value: Option<Vec<u8>>) -> *const c_char {
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
