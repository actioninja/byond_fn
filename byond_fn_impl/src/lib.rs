extern crate proc_macro;
use proc_macro::TokenStream;
use proc_macro2::TokenStream as TokenStream2;
use proc_macro_error::{abort, proc_macro_error};
use quote::quote;
use syn::spanned::Spanned;
use syn::{ItemFn, Signature};

#[proc_macro_error]
#[proc_macro_attribute]
pub fn byond_fn(args: TokenStream, input: TokenStream) -> TokenStream {
    byond_fn2(args.into(), input.into()).into()
}

fn byond_fn2(_args: TokenStream2, input: TokenStream2) -> TokenStream2 {
    let ItemFn { sig, block, .. }: ItemFn = syn::parse2(input).unwrap();

    let Signature { ident, .. } = sig;

    // needed function signature to export properly
    let fn_sig = quote! {
        #[no_mangle]
        #[allow(clippy::missing_safety_doc)]
        pub unsafe extern "C" fn #ident (
            argc: ::std::os::raw::c_int,
            argv: *const *const ::std::os::raw::c_char
        ) -> *const ::std::os::raw::c_char
    };

    let wrapped_block = quote! {
        let closure = || (#block);
    };

    let tokens = quote! {
        #fn_sig {
            #wrapped_block
            &0
        }
    };

    tokens
}

#[cfg(any(target_pointer_width = "32", feature = "allow_other_arch"))]
fn check_arch() -> bool {
    true
}

#[cfg(all(not(target_pointer_width = "32"), not(feature = "allow_other_arch")))]
fn check_arch() -> bool {
    false
}

#[cfg(test)]
mod test {
    #[test]
    fn expected_simple() {}
}
