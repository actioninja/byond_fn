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
