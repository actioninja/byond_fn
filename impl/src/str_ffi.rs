use proc_macro2::TokenStream;
use quote::{quote, ToTokens};
use syn::{FnArg, Signature};

use crate::{is_option_type, FFITokens};

fn return_type_token() -> TokenStream {
    quote! { *const ::std::os::raw::c_char }
}

fn args_tokens() -> TokenStream {
    quote! { argc: ::std::os::raw::c_int, argv: *const *const ::std::os::raw::c_char }
}

fn fn_body_tokens(sig: &Signature) -> TokenStream {
    let Signature { ident, inputs, .. } = sig;

    let min_args = inputs.iter().filter(|arg| !is_option_type(arg)).count();
    let max_args = inputs.len();
    let args_binding = inputs.iter().enumerate().map(|(num, arg)| {
        if let FnArg::Typed(arg) = arg {
            let arg = *arg.pat.clone();
            let arg_string = arg.to_token_stream().to_string();
            quote! {
                let #arg = match byond_fn::str_ffi::StrArg::map_arg(args.get(#num).map(|arg| *arg), #min_args, #max_args, #arg_string, #num) {
                    Ok(arg) => arg,
                    Err(err) => {
                        return byond_fn::str_ffi::byond_return(err);
                    },
                };
            }
        } else {
            panic!("Byond functions can't have self argument")
        }
    });

    let return_args = inputs.iter().map(|arg| {
        if let FnArg::Typed(arg) = arg {
            let pat = *arg.pat.clone();
            quote! { #pat }
        } else {
            panic!("Byond functions can't have self argument")
        }
    });

    let min_args_i32 = min_args as i32;
    let max_args_i32 = max_args as i32;
    let range_check = quote! {
        if argc < #min_args_i32  || argc > #max_args_i32 {
            return byond_fn::str_ffi::byond_return(byond_fn::str_ffi::TransportError::WrongArgCount {
                expected_min: #min_args,
                expected_max: #max_args,
                got: argc as usize,
            });
        }
    };

    let arg_stuff = if !inputs.is_empty() {
        quote! {
            #range_check
            let args = match byond_fn::str_ffi::parse_str_args(argc, argv) {
                Ok(args) => args,
                Err(err) => {
                    return byond_fn::str_ffi::byond_return(err);
                },
            };
            #(#args_binding)*
        }
    } else {
        quote! {}
    };

    quote! {
        #arg_stuff
        byond_fn::str_ffi::byond_return(super::#ident(#(#return_args),*))
    }
}

pub(crate) fn tokens(sig: &Signature) -> FFITokens {
    FFITokens {
        fn_args: args_tokens(),
        return_type: return_type_token(),
        fn_body: fn_body_tokens(sig),
    }
}
