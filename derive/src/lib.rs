#![no_std]
#![doc = include_str!("../README.md")]

extern crate alloc;
use proc_macro::TokenStream;
use quote::quote;
use syn::{
    punctuated::Punctuated, Data, DataStruct, DeriveInput, Expr, Fields, Ident, Meta, Token, Type,
};

#[derive(Default)]
struct FieldConfig {
    /// The expression provided in `#[config(default = ...)]`
    default: Option<Expr>,
    /// A flag for `#[config(skip_inherit)]`
    skip_inherit: bool,
}

struct ParsedField<'a> {
    ident: &'a Option<Ident>,
    ty: &'a Type,
    config: FieldConfig,
}

/// # Panics
/// Panics if the input is not a `DataStruct`.
#[proc_macro_derive(Config, attributes(config))]
pub fn config_derive(input: TokenStream) -> TokenStream {
    let ast: DeriveInput = syn::parse(input).unwrap();
    let struct_name = &ast.ident;
    let Data::Struct(DataStruct {
        fields: Fields::Named(fields),
        ..
    }) = &ast.data
    else {
        panic!("Config can only be derived for structs with named fields")
    };
    let parsed_fields: alloc::vec::Vec<_> = fields
        .named
        .iter()
        .map(|field| ParsedField {
            ident: &field.ident,
            ty: &field.ty,
            config: parse_field_config(&field.attrs).unwrap(),
        })
        .collect();
    let default_impl = generate_default_impl(struct_name, &parsed_fields);
    let inherit_impl = generate_inherit_impl(struct_name, &parsed_fields);
    quote! {
        #default_impl
        #inherit_impl
    }
    .into()
}

fn generate_default_impl(struct_name: &Ident, fields: &[ParsedField]) -> proc_macro2::TokenStream {
    let field_defaults = fields.iter().map(|field| {
        let field_name = field.ident;
        let field_ty = field.ty;
        let default_expr = field.config.default.as_ref().map_or_else(
            || quote! { <#field_ty as ::core::default::Default>::default() },
            |expr| quote! { #expr },
        );
        quote! { #field_name: #default_expr }
    });
    quote! {
        impl ::core::default::Default for #struct_name {
            fn default() -> Self {
                Self {
                    #(#field_defaults),*
                }
            }
        }
    }
}

fn generate_inherit_impl(struct_name: &Ident, fields: &[ParsedField]) -> proc_macro2::TokenStream {
    let field_inherits = fields.iter().map(|field| {
        let field_name = field.ident;
        if field.config.skip_inherit {
            quote! { #field_name: self.#field_name.clone() }
        } else {
            quote! {
                #field_name: ::inherit_config::InheritAble::inherit(&self.#field_name, &other.#field_name)
            }
        }
    });
    quote! {
        impl ::inherit_config::InheritAble for #struct_name {
            fn inherit(&self, other: &Self) -> Self {
                Self {
                    #(#field_inherits),*
                }
            }
        }
    }
}

fn parse_field_config(attrs: &[syn::Attribute]) -> syn::Result<FieldConfig> {
    let mut config = FieldConfig::default();
    for attr in attrs {
        if !attr.path().is_ident("config") {
            continue;
        }
        let nested = attr.parse_args_with(Punctuated::<Meta, Token![,]>::parse_terminated)?;
        for meta in nested {
            match meta {
                Meta::NameValue(nv) if nv.path.is_ident("default") => {
                    if config.default.is_some() {
                        return Err(syn::Error::new_spanned(
                            &nv.path,
                            "duplicate `default` attribute",
                        ));
                    }
                    config.default = Some(nv.value);
                }
                Meta::Path(path) if path.is_ident("skip_inherit") => {
                    config.skip_inherit = true;
                }
                _ => {
                    return Err(syn::Error::new_spanned(
                        meta,
                        "unrecognized config attribute",
                    ));
                }
            }
        }
    }
    Ok(config)
}
