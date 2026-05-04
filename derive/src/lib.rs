use proc_macro::TokenStream;
use quote::{format_ident, quote};
use syn::{parse_macro_input, Data, DataStruct, DeriveInput, Expr, Field, Fields, Type};

enum FieldStrategy {
    None,
    Literal(Expr),
    Expression(Expr),
    Nest,
}

struct FieldLogic {
    partial_field: proc_macro2::TokenStream,
    inherit: proc_macro2::TokenStream,
    simplify: proc_macro2::TokenStream,
    build: proc_macro2::TokenStream,
    default: proc_macro2::TokenStream,
}

/// 派生 `InheritConfig` 并自动生成 `Partial` 结构体。
#[proc_macro_derive(InheritConfig, attributes(config))]
pub fn inherit_config_derive(input: TokenStream) -> TokenStream {
    let ast = parse_macro_input!(input as DeriveInput);

    let Data::Struct(DataStruct {
        fields: Fields::Named(fields),
        ..
    }) = &ast.data
    else {
        return syn::Error::new_spanned(
            ast.ident,
            "InheritConfig macro only supports structs with named fields",
        )
        .to_compile_error()
        .into();
    };

    let mut partial_fields = Vec::new();
    let mut inherit_logic = Vec::new();
    let mut simplify_logic = Vec::new();
    let mut build_logic = Vec::new();
    let mut default_logic = Vec::new();

    for field in &fields.named {
        match process_field(field) {
            Ok(logic) => {
                partial_fields.push(logic.partial_field);
                inherit_logic.push(logic.inherit);
                simplify_logic.push(logic.simplify);
                build_logic.push(logic.build);
                default_logic.push(logic.default);
            }
            Err(e) => return e.to_compile_error().into(),
        }
    }

    generate_final_code(
        &ast.ident,
        &ast.vis,
        &partial_fields,
        &inherit_logic,
        &simplify_logic,
        &build_logic,
        &default_logic,
    )
}

/// 处理单个字段的配置逻辑
fn process_field(field: &Field) -> syn::Result<FieldLogic> {
    let f_name = field
        .ident
        .as_ref()
        .ok_or_else(|| syn::Error::new_spanned(field, "Field must have a name"))?;
    let f_ty = &field.ty;

    let strategy = parse_strategy(&field.attrs)?;

    let partial_field = match strategy {
        FieldStrategy::Nest => {
            let partial_ty = to_partial_type(f_ty);
            quote! {
                #[serde(skip_serializing_if = "Option::is_none")]
                pub #f_name: Option<#partial_ty>
            }
        }
        _ => {
            quote! {
                #[serde(skip_serializing_if = "Option::is_none")]
                pub #f_name: Option<#f_ty>
            }
        }
    };

    let (inherit, simplify, build, default) = match strategy {
        FieldStrategy::Nest => (
            quote! {
                match (&mut self.#f_name, &parent.#f_name) {
                    (Some(s), Some(p)) => s.inherit_from(p),
                    (None, Some(p)) => self.#f_name = Some(p.clone()),
                    _ => {}
                }
            },
            quote! {
                if let (Some(s), Some(p)) = (&mut self.#f_name, &parent.#f_name) {
                    s.simplify_from(p);
                }
                if self.#f_name == parent.#f_name {
                    self.#f_name = None;
                }
            },
            quote! { #f_name: self.#f_name.map(|c| c.build()).unwrap_or_else(|| Default::default()) },
            quote! { #f_name: Default::default() },
        ),
        FieldStrategy::Literal(ref expr) => (
            quote! { if self.#f_name.is_none() { self.#f_name = parent.#f_name.clone(); } },
            quote! { if self.#f_name == parent.#f_name { self.#f_name = None; } },
            quote! { #f_name: self.#f_name.unwrap_or(#expr) },
            quote! { #f_name: #expr },
        ),
        FieldStrategy::Expression(ref expr) => (
            quote! { if self.#f_name.is_none() { self.#f_name = parent.#f_name.clone(); } },
            quote! { if self.#f_name == parent.#f_name { self.#f_name = None; } },
            quote! { #f_name: self.#f_name.unwrap_or_else(|| #expr) },
            quote! { #f_name: #expr },
        ),
        FieldStrategy::None => (
            quote! { if self.#f_name.is_none() { self.#f_name = parent.#f_name.clone(); } },
            quote! { if self.#f_name == parent.#f_name { self.#f_name = None; } },
            quote! { #f_name: self.#f_name.unwrap_or_default() },
            quote! { #f_name: Default::default() },
        ),
    };

    Ok(FieldLogic {
        partial_field,
        inherit,
        simplify,
        build,
        default,
    })
}

/// 解析字段上的 `#[config(...)]` 属性
fn parse_strategy(attrs: &[syn::Attribute]) -> syn::Result<FieldStrategy> {
    let mut strategy = FieldStrategy::None;
    for attr in attrs {
        if !attr.path().is_ident("config") {
            continue;
        }
        attr.parse_nested_meta(|meta| {
            if meta.path.is_ident("default") {
                let expr: Expr = meta.value()?.parse()?;
                strategy = FieldStrategy::Literal(expr);
            } else if meta.path.is_ident("default_t") {
                let expr: Expr = meta.value()?.parse()?;
                strategy = FieldStrategy::Expression(expr);
            } else if meta.path.is_ident("nest") {
                strategy = FieldStrategy::Nest;
            }
            Ok(())
        })?;
    }
    Ok(strategy)
}

/// 组装最终的代码 `TokenStream`
fn generate_final_code(
    name: &syn::Ident,
    vis: &syn::Visibility,
    partial_fields: &[proc_macro2::TokenStream],
    inherit_logic: &[proc_macro2::TokenStream],
    simplify_logic: &[proc_macro2::TokenStream],
    build_logic: &[proc_macro2::TokenStream],
    default_logic: &[proc_macro2::TokenStream],
) -> TokenStream {
    let partial_name = format_ident!("Partial{}", name);

    let expanded = quote! {
        // 为原始结构体生成 Default 实现
        impl Default for #name {
            fn default() -> Self {
                Self { #(#default_logic),* }
            }
        }

        // 生成 Partial 结构体
        #[allow(clippy::derive_partial_eq_without_eq)]
        #[derive(Clone, Debug, Default, PartialEq, serde::Serialize, serde::Deserialize)]
        #vis struct #partial_name {
            #(#partial_fields),*
        }

        // 实现 ConfigLayer
        impl ::inherit_config::ConfigLayer for #partial_name {
            type Full = #name;

            fn inherit_from(&mut self, parent: &Self) {
                #(#inherit_logic)*
            }

            fn simplify_from(&mut self, parent: &Self) {
                #(#simplify_logic)*
            }

            fn build(self) -> Self::Full {
                #name {
                    #(#build_logic),*
                }
            }
        }
    };

    expanded.into()
}

// 辅助函数：将 Config 转为 PartialConfig
fn to_partial_type(ty: &Type) -> Type {
    let mut new_ty = ty.clone();
    if let Type::Path(type_path) = &mut new_ty {
        if let Some(segment) = type_path.path.segments.last_mut() {
            segment.ident = format_ident!("Partial{}", segment.ident);
        }
    }
    new_ty
}
