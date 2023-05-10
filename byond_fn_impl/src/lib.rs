#[cfg(feature = "ffi_v2")]
mod ffi_v2;
mod str_ffi;

extern crate proc_macro;
use proc_macro::TokenStream;
use proc_macro2::{Ident, TokenStream as TokenStream2};
use proc_macro_error::{abort, proc_macro_error};
use quote::quote;

use syn::spanned::Spanned;
use syn::{FnArg, ItemFn, Signature, Type};

pub(crate) struct FFITokens {
    fn_args: TokenStream2,
    args_transform: TokenStream2,
    return_type: TokenStream2,
    return_value: TokenStream2,
}

#[proc_macro_error]
#[proc_macro_attribute]
pub fn byond_fn(args: TokenStream, input: TokenStream) -> TokenStream {
    byond_fn2(args.into(), input.into()).into()
}

const STR_FFI_DESC: &str = "\"str\" (default): FFI with C Strings as the interop type";
const FFI_V2_DESC: &str =
    "\"v2\": New FFI Format added with BYOND 515 that uses `ByondType` as the FFI medium";

fn byond_fn2(args: TokenStream2, input: TokenStream2) -> TokenStream2 {
    let ItemFn { sig, block, .. }: ItemFn = syn::parse2(input).unwrap();

    let args: Ident = syn::parse2(args.clone()).unwrap_or(Ident::new("default", args.span()));

    let Signature { ident, inputs, .. } = sig;

    let FFITokens {
        fn_args,
        args_transform,
        return_type,
        return_value,
    } = match args.to_string().as_str() {
        "default" | "str" => str_ffi::tokens(inputs.clone().iter()),
        #[cfg(feature = "ffi_v2")]
        "v2" => {
            unimplemented!("Not yet implemented")
        }
        _ => {
            let first_line = format!("\n- {STR_FFI_DESC}");
            #[cfg(feature = "ffi_v2")]
            let second_line = format!("\n- {FFI_V2_DESC}");
            #[cfg(not(feature = "ffi_v2"))]
            let second_line = "";
            abort!(
                args,
                "\"{}\" is not a valid BYOND FFI function type",
                args.to_string();
                help = "VALID TYPES:{}{}", first_line, second_line

            )
        }
    };

    //verify optional params are at the tail of the sig
    let mut optional_encountered = false;
    for arg in inputs.iter() {
        let arg = match arg {
            FnArg::Receiver(_) => abort!(
                arg,
                "`byond_fn` only supports bare, non associated functions"
            ),
            FnArg::Typed(a) => a,
        };

        let arg_type = *arg.ty.clone();
        match arg_type {
            Type::Path(path) => {
                let path = path.path;
                if let Some(ident) = path.get_ident() {
                    if *ident == "Option" {
                        optional_encountered = true;
                    }
                }
            }
            _ => {
                if optional_encountered {
                    abort!(
                        arg,
                        "All optional parameters must be at the end of the signature"
                    );
                }
            }
        }
    }

    let fn_sig = quote! {
        #[no_mangle]
        #[allow(clippy::missing_safety_doc)]
        pub unsafe extern "C" fn #ident (
            #fn_args
        ) -> #return_type
    };

    let block_inner = block.stmts;

    let wrapped_block = quote! {
        let closure = || {
            #(#block_inner)*
        };
    };

    let tokens = quote! {
        #fn_sig {
            #args_transform
            #wrapped_block
            #return_value
        }
    };

    tokens
}

#[cfg(test)]
mod test {
    #[test]
    fn expected_simple() {}
}
