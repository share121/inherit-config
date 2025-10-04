use proc_macro::TokenStream;
use quote::quote;
use syn::{
    Data, DataStruct, DeriveInput, Expr, Fields, Ident, Meta, Token, Type, punctuated::Punctuated,
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

#[proc_macro_derive(Config, attributes(config))]
pub fn config_derive(input: TokenStream) -> TokenStream {
    let ast: DeriveInput = syn::parse(input).unwrap();
    let struct_name = &ast.ident;
    let fields = match &ast.data {
        Data::Struct(DataStruct {
            fields: Fields::Named(fields),
            ..
        }) => fields,
        _ => panic!("Config can only be derived for structs with named fields"),
    };
    let parsed_fields: Vec<_> = fields
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
        let default_expr = match &field.config.default {
            Some(expr) => quote! { #expr },
            None => quote! { <#field_ty as ::core::default::Default>::default() },
        };
        quote! {
            #field_name: #default_expr
        }
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
            quote! {
                #field_name: self.#field_name.clone()
            }
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

/// Parses the `#[config(...)]` attributes for a given field.
fn parse_field_config(attrs: &[syn::Attribute]) -> syn::Result<FieldConfig> {
    let mut config = FieldConfig::default();
    for attr in attrs {
        if !attr.path().is_ident("config") {
            continue;
        }
        let nested = attr.parse_args_with(Punctuated::<Meta, Token![,]>::parse_terminated)?;
        for meta in nested {
            match meta {
                // This matches `default = "..."`
                Meta::NameValue(nv) if nv.path.is_ident("default") => {
                    if config.default.is_some() {
                        return Err(syn::Error::new_spanned(
                            &nv.path,
                            "duplicate `default` attribute",
                        ));
                    }
                    config.default = Some(nv.value);
                }
                // This matches `skip_inherit`
                Meta::Path(path) if path.is_ident("skip_inherit") => {
                    config.skip_inherit = true;
                }
                // All other attributes are unsupported.
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
