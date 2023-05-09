use byond_fn_impl::byond_fn;

#[byond_fn]
pub fn example_byond_fn() {
    println!("Wow! It works!");
    println!("Here is more lines!");
    let x = "yeah, we can bind vars";
    println!("{x}");
}

#[byond_fn]
pub fn example_args(arg1: u8, arg2: u8) -> u8 {
    arg1 + arg2
}

#[test]
fn compiles() {}
