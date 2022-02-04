#![doc(
    html_logo_url = "https://raw.githubusercontent.com/max-m/rust-libretro/master/media/logo.png",
    html_favicon_url = "https://raw.githubusercontent.com/max-m/rust-libretro/master/media/favicon.png"
)]

use proc_macro::{self, TokenStream};
use quote::{quote, ToTokens};
use rust_libretro_sys::RETRO_NUM_CORE_OPTION_VALUES_MAX;
use syn::{
    braced, parenthesized,
    parse::{discouraged::Speculative, Parse, ParseStream, Result},
    parse2, parse_macro_input, parse_quote,
    punctuated::Punctuated,
    DeriveInput, LitByteStr, LitStr, Token,
};

mod util;
use util::*;

trait Concat<T> {
    fn concat(self) -> T;
}

#[derive(Debug)]
struct CoreOptionValue {
    value: LitStr,
    label: Option<LitStr>,
}

impl Parse for CoreOptionValue {
    fn parse(input: ParseStream) -> Result<Self> {
        let content;
        braced!(content in input);

        let value = content.parse()?;

        if !content.is_empty() {
            content.parse::<Token![,]>()?;
        }

        let label = if !content.is_empty() {
            Some(content.parse()?)
        } else {
            None
        };

        if !content.is_empty() {
            content.parse::<Token![,]>()?;
        }

        Ok(Self { value, label })
    }
}

#[derive(Debug)]
struct CoreOption {
    key: LitStr,
    desc: LitStr,
    info: LitStr,
    values: Vec<CoreOptionValue>,
    default_value: Option<LitStr>,
}

impl Parse for CoreOption {
    fn parse(input: ParseStream) -> Result<Self> {
        let key: LitStr = input.parse()?;
        input.parse::<Token![,]>()?;

        let desc: LitStr = input.parse()?;
        input.parse::<Token![,]>()?;

        let info: LitStr = input.parse()?;
        input.parse::<Token![,]>()?;

        let options_content;
        braced!(options_content in input);

        let default_value: Option<LitStr> = if !input.is_empty() {
            input.parse::<Token![,]>()?;
            input.parse()?
        } else {
            None
        };

        let mut values = Vec::new();
        while !options_content.is_empty() {
            let value = options_content.parse::<CoreOptionValue>()?;
            values.push(value);

            if !options_content.is_empty() {
                options_content.parse::<Token![,]>()?;
            }
        }

        Ok(Self {
            key,
            desc,
            info,
            values,
            default_value,
        })
    }
}

#[derive(Debug)]
struct CoreOptionV2 {
    key: LitStr,
    desc: LitStr,
    desc_categorized: Option<LitStr>,
    info: LitStr,
    info_categorized: Option<LitStr>,
    category_key: Option<LitStr>,
    values: Vec<CoreOptionValue>,
    default_value: Option<LitStr>,
}

impl Parse for CoreOptionV2 {
    fn parse(input: ParseStream) -> Result<Self> {
        let key: LitStr = input.parse()?;
        input.parse::<Token![,]>()?;

        let desc: LitStr = input.parse()?;
        input.parse::<Token![,]>()?;

        let desc_categorized: LitStr = input.parse()?;
        input.parse::<Token![,]>()?;

        let info: LitStr = input.parse()?;
        input.parse::<Token![,]>()?;

        let info_categorized: LitStr = input.parse()?;
        input.parse::<Token![,]>()?;

        let category_key: LitStr = input.parse()?;
        input.parse::<Token![,]>()?;

        let options_content;
        braced!(options_content in input);

        if !input.is_empty() {
            input.parse::<Token![,]>()?;
        }

        let default_value: Option<LitStr> = if !input.is_empty() {
            input.parse()?
        } else {
            None
        };

        let mut values = Vec::new();
        while !options_content.is_empty() {
            let value = options_content.parse::<CoreOptionValue>()?;
            values.push(value);

            if !options_content.is_empty() {
                options_content.parse::<Token![,]>()?;
            }
        }

        let ret = Ok(Self {
            key,
            desc,
            desc_categorized: Some(desc_categorized),
            info,
            info_categorized: Some(info_categorized),
            category_key: Some(category_key),
            values,
            default_value,
        });

        // allow trailing comma
        if input.is_empty() {
            return ret;
        }
        input.parse::<Token![,]>()?;

        ret
    }
}

impl From<CoreOption> for CoreOptionV2 {
    fn from(option: CoreOption) -> Self {
        Self {
            key: option.key,
            desc: option.desc,
            desc_categorized: None,
            info: option.info,
            info_categorized: None,
            category_key: None,
            values: option.values,
            default_value: option.default_value,
        }
    }
}

#[derive(Debug, Default)]
struct CoreOptions(Vec<CoreOptionV2>);

impl Parse for CoreOptions {
    fn parse(outer: ParseStream) -> Result<Self> {
        let input;
        parenthesized!(input in outer);

        let mut options = Self::default();

        while !input.is_empty() {
            let option;
            braced!(option in input);

            let core_option = {
                let fork = option.fork();
                if let Ok(option_v2) = fork.parse::<CoreOptionV2>() {
                    option.advance_to(&fork);
                    option_v2
                } else {
                    option.parse::<CoreOption>()?.into()
                }
            };

            options.0.push(core_option);

            // allow trailing comma
            if input.is_empty() {
                break;
            }
            input.parse::<Token![,]>()?;
        }

        Ok(options)
    }
}

impl Concat<CoreOptions> for Vec<CoreOptions> {
    fn concat(self) -> CoreOptions {
        CoreOptions(self.into_iter().map(|x| x.0).flatten().collect::<Vec<_>>())
    }
}

#[derive(Debug)]
struct CoreOptionCategory {
    key: LitStr,
    desc: LitStr,
    info: LitStr,
}

impl Parse for CoreOptionCategory {
    fn parse(input: ParseStream) -> Result<Self> {
        let key: LitStr = input.parse()?;
        input.parse::<Token![,]>()?;

        let desc: LitStr = input.parse()?;
        input.parse::<Token![,]>()?;

        let info: LitStr = input.parse()?;

        let ret = Ok(Self { key, desc, info });

        // allow trailing comma
        if input.is_empty() {
            return ret;
        }
        input.parse::<Token![,]>()?;

        ret
    }
}

#[derive(Debug, Default)]
struct CoreOptionCategories(Vec<CoreOptionCategory>);

impl Parse for CoreOptionCategories {
    fn parse(outer: ParseStream) -> Result<Self> {
        let input;
        parenthesized!(input in outer);

        let mut categories = Self::default();

        while !input.is_empty() {
            let category;
            braced!(category in input);

            let category = category.parse::<CoreOptionCategory>()?;

            categories.0.push(category);

            // allow trailing comma
            if input.is_empty() {
                break;
            }
            input.parse::<Token![,]>()?;
        }

        Ok(categories)
    }
}

impl Concat<CoreOptionCategories> for Vec<CoreOptionCategories> {
    fn concat(self) -> CoreOptionCategories {
        CoreOptionCategories(self.into_iter().map(|x| x.0).flatten().collect::<Vec<_>>())
    }
}

/// Used to define variables and core options for your [`rust_libretro::Core`] struct.
///
/// A struct that has been decorated with this attribute will have a `Self::set_core_options`
/// function which should be called in [`rust_libretro::Core::on_set_environment`].
///
/// Example usage:
/// ```ignore
/// #[derive(CoreOptions)]
/// #[options({
///     "foo_option_1",
///     "Speed hack coprocessor X",
///     "Provides increased performance at the expense of reduced accuracy",
///     {
///         { "false" },
///         { "true" },
///         { "unstable", "Turbo (Unstable)" },
///     },
///     "true"
/// }, {
///     "foo_option_2",
///     "Speed hack main processor Y",
///     "Provides increased performance at the expense of reduced accuracy",
///     {
///         { "false" },
///         { "true" },
///         { "unstable", "Turbo (Unstable)" },
///     },
/// })]
/// struct TestCore;
/// ```
///
/// TODO: Support structs with generic parameters.
/// TODO: Add V2 (category support) documentation
#[proc_macro_derive(DeriveCoreOptions, attributes(options, categories))]
pub fn derive_core_options(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);

    impl_derive_core_options(input)
}

fn impl_derive_core_options(input: DeriveInput) -> TokenStream {
    let name = &input.ident;
    let (impl_generics, ty_generics, where_clause) = input.generics.split_for_impl();
    let attrs = &input.attrs;

    let options = attrs
        .iter()
        .filter(|attr| attr.path.is_ident("options"))
        .map(|attr| -> Result<CoreOptions> { parse2(attr.tokens.clone()) })
        .collect::<Result<Vec<_>>>();

    let options = match options {
        Ok(options) => options.concat(),
        Err(err) => return TokenStream::from(err.to_compile_error()),
    };

    let categories = attrs
        .iter()
        .filter(|attr| attr.path.is_ident("categories"))
        .map(|attr| -> Result<CoreOptionCategories> { parse2(attr.tokens.clone()) })
        .collect::<Result<Vec<_>>>();

    let categories = match categories {
        Ok(categories) => categories.concat(),
        Err(err) => return TokenStream::from(err.to_compile_error()),
    };

    let option_count = options.0.len();
    let category_count = categories.0.len();

    fn lit_byte_str(lit: &LitStr) -> LitByteStr {
        let span = lit.span();
        let mut bytes = lit.value().into_bytes();
        bytes.push(0x00); // add terminating NULL byte

        LitByteStr::new(&bytes, span)
    }

    fn get_option_values(option: &CoreOptionV2) -> proc_macro2::TokenStream {
        let mut values = Vec::new();

        for index in 0..(RETRO_NUM_CORE_OPTION_VALUES_MAX as usize - 1) {
            values.push(if index < option.values.len() {
                let value = lit_byte_str(&option.values[index].value);

                if let Some(label) = &option.values[index].label {
                    let label = lit_byte_str(label);

                    quote! {
                        retro_core_option_value {
                            value: #value as *const u8 as *const libc::c_char,
                            label: #label as *const u8 as *const libc::c_char,
                        }
                    }
                } else {
                    quote! {
                        retro_core_option_value {
                            value: #value as *const u8 as *const libc::c_char,
                            label: 0 as *const libc::c_char,
                        }
                    }
                }
            } else {
                quote! {
                    retro_core_option_value {
                        value: 0 as *const libc::c_char,
                        label: 0 as *const libc::c_char,
                    }
                }
            });
        }

        values.push(quote! {
            retro_core_option_value {
                value: 0 as *const libc::c_char,
                label: 0 as *const libc::c_char,
            }
        });

        quote! {
            [ #(#values),* ]
        }
    }

    fn get_option_default_value(option: &CoreOptionV2) -> proc_macro2::TokenStream {
        if let Some(ref default_value) = option.default_value {
            let default_value = lit_byte_str(default_value);

            quote! {
                #default_value as *const u8 as *const libc::c_char
            }
        } else {
            quote! {
                0 as *const libc::c_char
            }
        }
    }

    let core_options = options
        .0
        .iter()
        .map(|option| {
            let key = lit_byte_str(&option.key);
            let desc = lit_byte_str(&option.desc);
            let info = lit_byte_str(&option.info);
            let values = get_option_values(option);
            let default_value = get_option_default_value(option);

            quote! {
                retro_core_option_definition {
                    key:    #key  as *const u8 as *const libc::c_char,
                    desc:   #desc as *const u8 as *const libc::c_char,
                    info:   #info as *const u8 as *const libc::c_char,
                    values: #values,
                    default_value: #default_value,
                }
            }
        })
        .collect::<Vec<_>>();

    let core_variables = options
        .0
        .iter()
        .map(|option| {
            let key = lit_byte_str(&option.key);

            let value = &format!(
                "{}; {}",
                &option.desc.value(),
                option
                    .values
                    .iter()
                    .map(|value| value.value.value())
                    .collect::<Vec<_>>()
                    .join("|")
            )
            .into_bytes();
            let value = LitByteStr::new(value, option.desc.span());

            quote! {
                retro_variable {
                    key:   #key   as *const u8 as *const libc::c_char,
                    value: #value as *const u8 as *const libc::c_char,
                }
            }
        })
        .collect::<Vec<_>>();

    let core_options_v2 = options
        .0
        .iter()
        .map(|option| {
            let key = lit_byte_str(&option.key);
            let desc = lit_byte_str(&option.desc);
            let info = lit_byte_str(&option.info);
            let values = get_option_values(option);
            let default_value = get_option_default_value(option);

            let desc_categorized = lit_byte_str(
                option
                    .desc_categorized
                    .as_ref()
                    .unwrap_or(&LitStr::new("", proc_macro2::Span::call_site())),
            );
            let info_categorized = lit_byte_str(
                option
                    .info_categorized
                    .as_ref()
                    .unwrap_or(&LitStr::new("", proc_macro2::Span::call_site())),
            );
            let category_key = lit_byte_str(
                option
                    .category_key
                    .as_ref()
                    .unwrap_or(&LitStr::new("", proc_macro2::Span::call_site())),
            );

            quote! {
                retro_core_option_v2_definition {
                    key:  #key  as *const u8 as *const libc::c_char,
                    desc: #desc as *const u8 as *const libc::c_char,
                    info: #info as *const u8 as *const libc::c_char,

                    desc_categorized: #desc_categorized as *const u8 as *const libc::c_char,
                    info_categorized: #info_categorized as *const u8 as *const libc::c_char,
                    category_key:     #category_key     as *const u8 as *const libc::c_char,

                    values: #values,
                    default_value: #default_value,
                }
            }
        })
        .collect::<Vec<_>>();

    let core_option_categories = categories
        .0
        .iter()
        .map(|category| {
            let key = lit_byte_str(&category.key);
            let desc = lit_byte_str(&category.desc);
            let info = lit_byte_str(&category.info);

            quote! {
                retro_core_option_v2_category {
                    key:    #key  as *const u8 as *const libc::c_char,
                    desc:   #desc as *const u8 as *const libc::c_char,
                    info:   #info as *const u8 as *const libc::c_char,
                }
            }
        })
        .collect::<Vec<_>>();

    let expanded = quote! {
        impl #impl_generics #name #ty_generics #where_clause {
            #[doc(hidden)]
            const __RETRO_CORE_OPTIONS: [retro_core_option_definition; #option_count + 1] = [
                #(#core_options,)*

                // List terminator
                retro_core_option_definition {
                    key:    0 as *const libc::c_char,
                    desc:   0 as *const libc::c_char,
                    info:   0 as *const libc::c_char,
                    values: [retro_core_option_value {
                        value: 0 as *const libc::c_char,
                        label: 0 as *const libc::c_char,
                    }; #RETRO_NUM_CORE_OPTION_VALUES_MAX as usize],
                    default_value: 0 as *const libc::c_char,
                }
            ];

            #[doc(hidden)]
            const __RETRO_CORE_VARIABLES: [retro_variable; #option_count + 1] = [
                #(#core_variables,)*

                // List terminator
                retro_variable {
                    key:   0 as *const libc::c_char,
                    value: 0 as *const libc::c_char,
                }
            ];

            #[doc(hidden)]
            const __RETRO_CORE_OPTION_V2_CATEGORIES: [retro_core_option_v2_category; 1 + #category_count] = [
                #(#core_option_categories,)*

                retro_core_option_v2_category {
                    key: 0 as *const libc::c_char,
                    desc: 0 as *const libc::c_char,
                    info: 0 as *const libc::c_char,
                }
            ];

            #[doc(hidden)]
            const __RETRO_CORE_OPTION_V2_DEFINITIONS: [retro_core_option_v2_definition; #option_count + 1] = [
                #(#core_options_v2,)*

                // List terminator
                retro_core_option_v2_definition {
                    key: 0 as *const libc::c_char,
                    desc: 0 as *const libc::c_char,
                    desc_categorized: 0 as *const libc::c_char,
                    info: 0 as *const libc::c_char,
                    info_categorized: 0 as *const libc::c_char,
                    category_key: 0 as *const libc::c_char,
                    values: [retro_core_option_value {
                        value: 0 as *const libc::c_char,
                        label: 0 as *const libc::c_char,
                    }; 128],
                    default_value: 0 as *const libc::c_char,
                }
            ];

            #[doc(hidden)]
            const __RETRO_CORE_OPTIONS_V2: retro_core_options_v2 = retro_core_options_v2 {
                /// HERE BE DRAGONS, but mutable references are not allowed
                categories: &Self::__RETRO_CORE_OPTION_V2_CATEGORIES as *const _ as *mut _,
                /// HERE BE DRAGONS, but mutable references are not allowed
                definitions: &Self::__RETRO_CORE_OPTION_V2_DEFINITIONS as *const _ as *mut _,
            };

            /// For some reason the call to `supports_set_core_options` only works on the initial call of `on_set_environment`.
            /// On subsequent calls of `on_set_environment` querying `RETRO_ENVIRONMENT_GET_CORE_OPTIONS_VERSION` returns NULL pointers.
            unsafe fn set_core_options(ctx: &SetEnvironmentContext) -> bool {
                let gctx: GenericContext = ctx.into();

                match gctx.get_core_options_version() {
                    n if n >= 2 => ctx.set_core_options_v2(&Self::__RETRO_CORE_OPTIONS_V2),
                    n if n >= 1 => ctx.set_core_options(&Self::__RETRO_CORE_OPTIONS),
                    _ => ctx.set_variables(&Self::__RETRO_CORE_VARIABLES)
                }
            }
        }
    };

    TokenStream::from(expanded)
}

/// Marks a function as unstable and guards it behind a feature flag.
///
/// Feature names are accepted as either `#[unstable(feature_name)]` or `#[unstable(feature = "name")]`
/// If no name was given `unstable` is assumed.
///
/// The defining crate is allowed to use functions marked as unstable even when the feature is disabled.
#[proc_macro_attribute]
pub fn unstable(args: TokenStream, input: TokenStream) -> TokenStream {
    let args = parse_macro_input!(args as syn::AttributeArgs);
    let mut item = parse_macro_input!(input as syn::Item);

    let feature_name = {
        let mut name = "unstable".to_owned();

        for arg in args.iter() {
            if let syn::NestedMeta::Lit(syn::Lit::Str(custom_name)) = arg {
                name = format!("unstable-{}", custom_name.value());
                break;
            } else if let syn::NestedMeta::Meta(syn::Meta::NameValue(named_value)) = arg {
                if let syn::Lit::Str(custom_name) = &named_value.lit {
                    name = format!("unstable-{}", custom_name.value());
                    break;
                }
            }
        }

        name
    };

    if let syn::Item::Fn(ref mut item) = item {
        // Mark the function as unsafe
        item.sig.unsafety = Some(syn::parse_quote!(unsafe));
    }

    if is_public(&item) {
        prepend_doc(&mut item, "<span class='stab unstable'>Unstable</span>");

        let unstable_doc = format!(
            "# This feature is unstable and guarded by the `{}` feature flag.\
            \n\
            Please be advised that this feature might change without further notice\
            and no guarantees about its stability can be made.",
            feature_name
        );

        push_attr(
            &mut item,
            syn::parse_quote! {
                #[doc = #unstable_doc]
            },
        );

        let mut private_item = item.clone();
        if let Some(vis) = get_visibility_mut(&mut private_item) {
            *vis = syn::parse_quote!(pub(crate));
        }

        return TokenStream::from(quote! {
            #[cfg(feature = #feature_name)]
            #[allow(unused_unsafe)]
            #item

            #[cfg(not(feature = #feature_name))]
            #[allow(unused_unsafe)]
            #[allow(dead_code)]
            #private_item
        });
    }

    item.into_token_stream().into()
}

#[proc_macro_attribute]
pub fn context(args: TokenStream, input: TokenStream) -> TokenStream {
    let ctx_name = parse_macro_input!(args as syn::Ident);

    let item = parse_macro_input!(input as syn::ItemFn);
    let mut fun = item.clone();

    // Mark functions as safe in this context
    fun.sig.unsafety = None;

    let mut inputs: Punctuated<syn::FnArg, Token![,]> = Punctuated::new();
    inputs.push(parse_quote!(&self));

    // Remove the environment callback argument
    for arg in fun.sig.inputs.iter().filter(|input| {
        if let syn::FnArg::Typed(arg) = input {
            if let syn::Type::Path(ty) = &*arg.ty {
                if ty.path.is_ident("retro_environment_t")
                    || ty.path.segments.last().unwrap().ident == "retro_environment_t"
                {
                    return false;
                }
            }
        }

        true
    }) {
        inputs.push(arg.clone());
    }

    // Remove the `context` attribute
    fun.attrs = fun
        .attrs
        .into_iter()
        .filter(|attr| attr.path.segments.last().unwrap().ident != "context")
        .collect();

    // Replace the function arguments
    fun.sig.inputs = inputs;

    // Create the function call
    let fun_name = &fun.sig.ident;
    let mut fun_call_args: Punctuated<syn::Expr, Token![,]> = Punctuated::new();
    fun_call_args.push(parse_quote!(*self.environment_callback));

    // Skip the `self` argument
    for arg in fun.sig.inputs.iter().skip(1) {
        if let syn::FnArg::Typed(arg) = arg {
            if let syn::Pat::Ident(pat_ident) = &*arg.pat {
                let ident = &pat_ident.ident;
                fun_call_args.push(parse_quote!(#ident));
            }
        }
    }

    fun.block = parse_quote! {{
        unsafe {
            environment::#fun_name(#fun_call_args)
        }
    }};

    let ctx_impl = quote! {
        #item

        impl #ctx_name<'_> {
            #[inline]
            #[allow(deprecated)]
            #fun
        }
    };

    TokenStream::from(ctx_impl)
}
