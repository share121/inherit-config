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
    from_full: proc_macro2::TokenStream,
}

struct ParsedFieldConfig {
    strategy: FieldStrategy,
    partial_attrs: Vec<proc_macro2::TokenStream>,
}

/// 派生 `InheritConfig` 并自动生成 `Partial` 结构体。
///
/// ## 可用属性
///
/// - `#[config(default = <literal>)]` 用于简单的字面量或常量（如 `32`, `true`, `"str"`）。宏会将其转化为 `.unwrap_or(<literal>)` 以获得最佳性能。
/// - `#[config(default_t = <expression>)]` 用于涉及内存分配或需要执行函数调用的复杂类型（如 `String::new()`, `vec![]`, `dirs::home_dir().unwrap()`）。宏会将其转化为 `.unwrap_or_else(|| <expression>)`，**实现真正的惰性求值**。
/// - `#[config(nest)]` 用于标记嵌套配置结构体。宏会自动对该字段进行递归地 `inherit_from`、`simplify_from` 和 `build` 操作。注意被嵌套的结构体也必须 Derive 了 `InheritConfig`。
/// - `#[config(partial_attr(...))]` 用于在影子结构体上传递自定义属性如 `#[config(partial_attr(serde(with = "humantime_serde")))]`。
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

    // 解析结构体级别的 partial_attr
    let mut struct_partial_attrs = Vec::new();
    for attr in &ast.attrs {
        if attr.path().is_ident("config") {
            if let Err(e) = attr.parse_nested_meta(|meta| {
                if meta.path.is_ident("partial_attr") {
                    let content;
                    syn::parenthesized!(content in meta.input);
                    let tokens: proc_macro2::TokenStream = content.parse()?;
                    struct_partial_attrs.push(tokens);
                }
                Ok(())
            }) {
                return e.to_compile_error().into();
            }
        }
    }

    let mut partial_fields = Vec::new();
    let mut inherit_logic = Vec::new();
    let mut simplify_logic = Vec::new();
    let mut build_logic = Vec::new();
    let mut default_logic = Vec::new();
    let mut from_full_logic = Vec::new();

    for field in &fields.named {
        match process_field(field) {
            Ok(logic) => {
                partial_fields.push(logic.partial_field);
                inherit_logic.push(logic.inherit);
                simplify_logic.push(logic.simplify);
                build_logic.push(logic.build);
                default_logic.push(logic.default);
                from_full_logic.push(logic.from_full);
            }
            Err(e) => return e.to_compile_error().into(),
        }
    }

    generate_final_code(
        &ast.ident,
        &ast.vis,
        &struct_partial_attrs,
        &partial_fields,
        &inherit_logic,
        &simplify_logic,
        &build_logic,
        &default_logic,
        &from_full_logic,
    )
}

/// 处理单个字段的配置逻辑
fn process_field(field: &Field) -> syn::Result<FieldLogic> {
    let f_name = field
        .ident
        .as_ref()
        .ok_or_else(|| syn::Error::new_spanned(field, "Field must have a name"))?;
    let f_ty = &field.ty;

    let parsed = parse_field_config(&field.attrs)?;
    let strategy = parsed.strategy;
    let p_attrs = parsed.partial_attrs;

    // 自动透传文档注释 ///
    let mut doc_attrs = Vec::new();
    for attr in &field.attrs {
        if attr.path().is_ident("doc") {
            doc_attrs.push(attr.clone());
        }
    }

    let partial_field = if matches!(strategy, FieldStrategy::Nest) {
        let partial_ty = to_partial_type(f_ty);
        quote! {
            #(#doc_attrs)*
            #( #[#p_attrs] )*
            #[serde(skip_serializing_if = "Option::is_none")]
            pub #f_name: Option<#partial_ty>
        }
    } else {
        quote! {
            #(#doc_attrs)*
            #( #[#p_attrs] )*
            #[serde(skip_serializing_if = "Option::is_none")]
            pub #f_name: Option<#f_ty>
        }
    };

    let from_full = if matches!(strategy, FieldStrategy::Nest) {
        let partial_ty = to_partial_type(f_ty);
        quote! { #f_name: Some(#partial_ty::from_full(full.#f_name)) }
    } else {
        quote! { #f_name: Some(full.#f_name) }
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
                if self.#f_name == parent.#f_name || self.#f_name == Some(Default::default()) {
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
        from_full,
    })
}

/// 解析字段上的 `#[config(...)]` 属性
fn parse_field_config(attrs: &[syn::Attribute]) -> syn::Result<ParsedFieldConfig> {
    let mut strategy = FieldStrategy::None;
    let mut partial_attrs = Vec::new();

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
            } else if meta.path.is_ident("partial_attr") {
                let content;
                syn::parenthesized!(content in meta.input);
                let tokens: proc_macro2::TokenStream = content.parse()?;
                partial_attrs.push(tokens);
            }
            Ok(())
        })?;
    }
    Ok(ParsedFieldConfig {
        strategy,
        partial_attrs,
    })
}

/// 组装最终的代码 `TokenStream`
#[allow(clippy::too_many_arguments)]
fn generate_final_code(
    name: &syn::Ident,
    vis: &syn::Visibility,
    struct_partial_attrs: &[proc_macro2::TokenStream],
    partial_fields: &[proc_macro2::TokenStream],
    inherit_logic: &[proc_macro2::TokenStream],
    simplify_logic: &[proc_macro2::TokenStream],
    build_logic: &[proc_macro2::TokenStream],
    default_logic: &[proc_macro2::TokenStream],
    from_full_logic: &[proc_macro2::TokenStream],
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
        #( #[#struct_partial_attrs] )*
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

            fn from_full(full: Self::Full) -> Self {
                Self {
                    #(#from_full_logic),*
                }
            }
        }

        impl From<#name> for #partial_name {
            fn from(full: #name) -> Self {
                <Self as ::inherit_config::ConfigLayer>::from_full(full)
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
