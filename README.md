# byond_fn

<!-- cargo-rdme start -->

A macro crate for defining functions callable from BYOND easily and ergonomically

### Usage

Basic usage is as simple as:
```rust
use byond_fn::byond_fn;

#[byond_fn]
pub fn add(arg1: u8, arg2: u8) -> u8 {
    arg1 + arg2
}
```
This will generate a extern "C" function called `add` that can be called from BYOND:

`call_ext("example_name.dll", "add")("2", "2") // returns 4`


### Optional Parameters

If a parameter is an `Option`, it will be optional to call from BYOND.

All optional parameters must be at the end of the parameter list.

<!-- cargo-rdme end -->
