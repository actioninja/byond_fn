use crate::FFITokens;
use proc_macro2::TokenStream;
use quote::{quote, quote_spanned};
use syn::spanned::Spanned;
use syn::FnArg;

fn return_type_token() -> TokenStream {
    quote! { *const ::std::os::raw::c_char }
}

fn args_tokens() -> TokenStream {
    quote! { argc: ::std::os::raw::c_int, argv: *const *const ::std::os::raw::c_char }
}

fn args_transform<'a>(fn_args: impl Iterator<Item = &'a FnArg>) -> TokenStream {
    let args_bind = fn_args.enumerate().map(|(num, arg)| {
        let arg = match arg {
            FnArg::Receiver(_) => panic!("Byond functions can't have self argument"),
            FnArg::Typed(arg) => arg,
        };

        quote_spanned! { arg.span() =>
            let #arg = byond_fn::str_ffi::StrArg::from_arg(args[#num].clone());
        }
    });
    quote! {
        let args = byond_fn::str_ffi::parse_str_args(argc, argv);
        #(#args_bind)*
    }
}

fn return_value_tokens() -> TokenStream {
    quote! { byond_fn::str_ffi::byond_return(closure()) }
}

pub(crate) fn tokens<'a>(fn_args: impl Iterator<Item = &'a FnArg>) -> FFITokens {
    FFITokens {
        fn_args: args_tokens(),
        return_type: return_type_token(),
        args_transform: args_transform(fn_args),
        return_value: return_value_tokens(),
    }
}
