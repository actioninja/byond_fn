use proc_macro2::TokenStream;
use quote::{quote, quote_spanned};
use syn::{FnArg, Signature};
use syn::punctuated::Punctuated;
use syn::spanned::Spanned;
use syn::token::Comma;

use crate::FFITokens;

fn return_type_token() -> TokenStream {
    quote! { *const ::std::os::raw::c_char }
}

fn args_tokens() -> TokenStream {
    quote! { argc: ::std::os::raw::c_int, argv: *const *const ::std::os::raw::c_char }
}

fn args_transform(fn_args: &Punctuated<FnArg, Comma>) -> TokenStream {
    let args_bind = fn_args.iter().enumerate().map(|(num, arg)| {
        let arg = match arg {
            FnArg::Receiver(_) => panic!("Byond functions can't have self argument"),
            FnArg::Typed(arg) => arg,
        };

        quote_spanned! { arg.span() =>
            let #arg = byond_fn::str_ffi::StrArg::from_arg(args.get(#num));
        }
    });
    quote! {
        let args = byond_fn::str_ffi::parse_str_args(argc, argv);
        #(#args_bind)*
    }
}

fn fn_body_tokens(sig: &Signature) -> TokenStream {
    let Signature { ident, inputs, .. } = sig;

    let args_binding = inputs.iter().enumerate().map(|(num, arg)| {
        if let FnArg::Typed(arg) = arg {
            quote! {
                let #arg = match byond_fn::str_ffi::StrArg::from_arg(args.get(#num).map(|arg| arg.clone())) {
                    Ok(arg) => arg,
                    Err(err) => {
                        return byond_fn::str_ffi::byond_return(err.to_string()).unwrap();
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

    quote! {
        let args = byond_fn::str_ffi::parse_str_args(argc, argv);
        #(#args_binding)*
        byond_fn::str_ffi::byond_return(super::#ident(#(#return_args),*)).unwrap_or_else(|err| {
            byond_fn::str_ffi::byond_return(err.to_string()).unwrap()
        })
    }
}

fn check_range_token() -> TokenStream {
    quote! {

        if argc < min_args  || argc > max_args {
            return byond_fn::str_ffi::byond_return(byond_fn::str_ffi::TransportError::WrongArgCount.to_string()).unwrap();
        }
    }
}

pub(crate) fn tokens(sig: &Signature) -> FFITokens {
    FFITokens {
        fn_args: args_tokens(),
        return_type: return_type_token(),
        fn_body: fn_body_tokens(sig),
        range_check: check_range_token(),
    }
}
