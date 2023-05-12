extern crate proc_macro;

use proc_macro::TokenStream;

use proc_macro2::{Ident, TokenStream as TokenStream2};
use proc_macro_error::{abort, proc_macro_error};
use quote::quote;
use syn::spanned::Spanned;
use syn::{FnArg, ItemFn, Signature, Type};

#[cfg(feature = "ffi_v2")]
mod ffi_v2;
mod str_ffi;

pub(crate) struct FFITokens {
    fn_args: TokenStream2,
    return_type: TokenStream2,
    fn_body: TokenStream2,
}

fn is_option_type(arg: &FnArg) -> bool {
    match arg {
        FnArg::Receiver(_) => abort!(arg.span(), "byond_fn can't have self argument"),
        FnArg::Typed(arg) => match *arg.ty {
            Type::Path(ref path) => path.path.segments.last().unwrap().ident == "Option",
            _ => false,
        },
    }
}

#[proc_macro_error]
#[proc_macro_attribute]
pub fn byond_fn(args: TokenStream, input: TokenStream) -> TokenStream {
    byond_fn2(args.into(), input.into()).into()
}

const STR_FFI_DESC: &str = "\"str\" (default): FFI with C Strings as the interop type";
const FFI_V2_DESC: &str =
    "\"v2\": New FFI Format added with BYOND 515 that uses `ByondType` as the FFI medium";

fn byond_fn2(proc_args: TokenStream2, input: TokenStream2) -> TokenStream2 {
    let original_fn: ItemFn = syn::parse2(input).unwrap();

    let proc_args: Ident =
        syn::parse2(proc_args.clone()).unwrap_or(Ident::new("default", proc_args.span()));

    let sig = &original_fn.sig;

    let Signature { ident, inputs, .. } = &sig;

    //verify optional params are at the tail of the sig
    let mut optional_encountered = false;
    for arg in inputs.iter() {
        if optional_encountered && !is_option_type(arg) {
            abort!(
                arg.span(),
                "Optional arguments must be at the end of the function signature"
            );
        } else {
            optional_encountered = is_option_type(arg);
        }
    }

    let mangled_name = Ident::new(
        format!("__byond_fn_{}", ident.to_string()).as_str(),
        ident.span(),
    );

    let FFITokens {
        fn_args,
        return_type,
        fn_body,
    } = str_ffi::tokens(sig);

    quote! {
        #original_fn
        mod #mangled_name {
            #[no_mangle]
            pub unsafe extern "C" fn #ident(#fn_args) -> #return_type {
                #fn_body
            }
        }
    }
}

#[cfg(test)]
mod test {
    use quote::quote;

    use super::*;

    #[test]
    fn is_optional_valid() {
        let arg: FnArg = syn::parse2(quote! { foo: i32 }).unwrap();
        assert!(!is_option_type(&arg));

        let arg: FnArg = syn::parse2(quote! { foo: Option<i32> }).unwrap();
        assert!(is_option_type(&arg));
    }
}
