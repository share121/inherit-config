# inherit-config

[![GitHub last commit](https://img.shields.io/github/last-commit/share121/inherit-config/master)](https://github.com/share121/inherit-config/commits/master)
[![Test](https://github.com/share121/inherit-config/workflows/Test/badge.svg)](https://github.com/share121/inherit-config/actions)
[![MIT License](https://img.shields.io/badge/license-MIT-blue.svg)](https://github.com/share121/inherit-config/blob/master/LICENSE)

一个 Rust 库，提供了一个派生宏（derive macro）来轻松实现可继承的配置模式。

## 概述

在许多应用程序中，配置是通过多个层级来确定的。例如，可能有一个全局的默认配置，然后是一个用户级的配置，最后是一个项目级的配置，每一层都可以覆盖前一层的值。

`inherit-config` 通过提供一个 `#[derive(Config)]` 宏来简化这个过程，该宏会自动为您的结构体实现 `Default` 和一个自定义的 `InheritAble` trait。这使得您可以轻松地合并多个不同层级的配置对象。

### 核心组件

- **`InheritAble` Trait**: 一个简单的 trait，定义了 `inherit(&self, other: &Self) -> Self` 方法，用于实现继承逻辑。
- **`ConfigField<T>` Enum**: 一个辅助枚举，用于表示一个配置项的三种状态：
  - `Inherit`: 字段应从父配置继承其值。这是默认状态。
  - `Set(T)`: 字段有一个明确设置的值，它将覆盖父配置。
  - `Unset`: 字段被显式地标记为“未设置”，它将覆盖父配置。
- **`#[derive(Config)]`**: 过程宏，它为您的结构体生成 `Default` 和 `InheritAble` 的实现代码。

## 安装

将以下内容添加到您的 `Cargo.toml` 文件中：

```toml
[dependencies]
inherit-config = "0.1.0" # 核心 trait 和类型
inherit-config-derive = "0.1.0" # 派生宏
```

## 使用方法

```rust
use inherit_config::{ConfigField, InheritAble};
use inherit_config_derive::Config;

#[derive(Clone, Config)]
struct AppConfig {
    #[config(default = ConfigField::Unset)]
    proxy: ConfigField<String>,

    #[config(default = ConfigField::Set("Referer: https://example.com/"))]
    headers: ConfigField<String>,

    #[config(default = Some(16))]
    font_size: Option<u32>,

    // 对于没有实现 InheritAble 的类型，您需要手动实现 inherit 方法或者添加 `skip_inherit` 属性。
    #[config(default = 1, skip_inherit)]
    foo: i32,
}

fn main() {
    // 1. 创建一个全局配置
    let global_config = AppConfig::default();

    // 2. 创建一个“局部”配置
    let mut local_config = AppConfig {
        proxy: ConfigField::Inherit,
        headers: ConfigField::Unset,
        font_size: ConfigField::Set(12),
        foo: 2,
    };

    // 3. 将局部配置与全局配置合并
    let final_config = local_config.inherit(&global_config);

    // 4. 验证结果
    assert_eq!(final_config, AppConfig {
        proxy: ConfigField::Unset, // 继承自 `global_config`
        headers: ConfigField::Unset, // 覆盖 `global_config` 的值
        font_size: ConfigField::Set(12), // 覆盖 `global_config` 的值
        foo: 2, // 覆盖 `global_config` 的值
    });
}
```
