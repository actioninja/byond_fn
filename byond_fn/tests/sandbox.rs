#![feature(prelude_import)]
#[prelude_import]
use std::prelude::rust_2021::*;
#[macro_use]
extern crate std;
use byond_fn_impl::byond_fn;
#[no_mangle]
#[allow(clippy::missing_safety_doc)]
pub unsafe extern "C" fn example_byond_fn(
    argc: ::std::os::raw::c_int,
    argv: *const *const ::std::os::raw::c_char,
) -> *const ::std::os::raw::c_char {
    ()
}
extern crate test;
#[cfg(test)]
#[rustc_test_marker = "compiles"]
pub const compiles: test::TestDescAndFn = test::TestDescAndFn {
    desc: test::TestDesc {
        name: test::StaticTestName("compiles"),
        ignore: false,
        ignore_message: ::core::option::Option::None,
        source_file: "byond_fn\\tests\\basic_compiles.rs",
        start_line: 9usize,
        start_col: 4usize,
        end_line: 9usize,
        end_col: 12usize,
        compile_fail: false,
        no_run: false,
        should_panic: test::ShouldPanic::No,
        test_type: test::TestType::IntegrationTest,
    },
    testfn: test::StaticTestFn(|| test::assert_test_result(compiles())),
};
fn compiles() {}
#[rustc_main]
pub fn main() -> () {
    extern crate test;
    test::test_main_static(&[&compiles])
}
