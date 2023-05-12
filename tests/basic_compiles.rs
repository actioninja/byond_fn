use byond_fn_impl::byond_fn;

#[byond_fn]
pub fn example_byond_fn() {
    let x = "yeah, we can bind vars";
    println!("{x}");
}

#[byond_fn]
pub fn add(arg1: u8, arg2: u8) -> u8 {
    arg1 + arg2
}

#[byond_fn]
pub fn example_optional_params(arg1: u8, arg2: Option<u8>) -> u8 {
    arg1 + arg2.unwrap_or(0)
}

#[byond_fn]
pub fn shouldnt_compile(arg1: Option<u8>, arg2: u8) -> u8 {
    arg1.unwrap_or(0) + arg2
}

#[test]
fn compiles() {}
