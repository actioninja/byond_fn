use byond_fn_impl::byond_fn;

#[byond_fn]
pub fn example_byond_fn() {
    println!("Wow! It works!");
}

#[test]
fn compiles() {}
