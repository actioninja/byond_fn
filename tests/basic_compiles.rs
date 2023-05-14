#![warn(clippy::pedantic)]

use byond_fn_impl::byond_fn;

#[byond_fn]
pub fn example_byond_fn() {
    let x = "yeah, we can bind vars";
    println!("{x}");
}

pub fn add(arg1: u8, arg2: u8) -> u8 {
    arg1 + arg2
}

mod __byond_fn_add {
    #[no_mangle]
    pub unsafe extern "C" fn add(
        argc: ::std::os::raw::c_int,
        argv: *const *const ::std::os::raw::c_char,
    ) -> *const ::std::os::raw::c_char {
        if argc != 2i32 {
            return byond_fn::str_ffi::byond_return(
                byond_fn::str_ffi::TransportError::WrongArgCount {
                    expected_min: 2usize,
                    expected_max: 2usize,
                    got: argc as usize,
                },
            );
        }
        let args = match byond_fn::str_ffi::parse_str_args(argc, argv) {
            Ok(args) => args,
            Err(err) => {
                return byond_fn::str_ffi::byond_return(err);
            }
        };
        let arg1 = match byond_fn::str_ffi::StrArg::map_arg(
            args.get(0usize).copied(),
            2usize,
            2usize,
            "arg1",
            0usize,
        ) {
            Ok(arg) => arg,
            Err(err) => {
                return byond_fn::str_ffi::byond_return(err);
            }
        };
        let arg2 = match byond_fn::str_ffi::StrArg::map_arg(
            args.get(1usize).copied(),
            2usize,
            2usize,
            "arg2",
            1usize,
        ) {
            Ok(arg) => arg,
            Err(err) => {
                return byond_fn::str_ffi::byond_return(err);
            }
        };
        byond_fn::str_ffi::byond_return(super::add(arg1, arg2))
    }
}

#[byond_fn]
pub fn example_optional_params(arg1: u8, arg2: Option<u8>) -> u8 {
    arg1 + arg2.unwrap_or(0)
}

#[test]
fn compiles() {}
