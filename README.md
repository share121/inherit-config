# inherit-config

[![GitHub last commit](https://img.shields.io/github/last-commit/share121/inherit-config/master)](https://github.com/share121/inherit-config/commits/master)
[![Test](https://github.com/share121/inherit-config/workflows/Test/badge.svg)](https://github.com/share121/inherit-config/actions)
[![Latest version](https://img.shields.io/crates/v/inherit-config.svg)](https://crates.io/crates/inherit-config)
[![Documentation](https://docs.rs/inherit-config/badge.svg)](https://docs.rs/inherit-config)
[![License](https://img.shields.io/crates/l/inherit-config.svg)](https://github.com/share121/inherit-config/blob/master/LICENSE)

`inherit-config` 是一个专为 Rust 打造的轻量级、零样板代码的**层级配置管理库**。

它通过派生宏自动实现 **Partial Struct Pattern（部分结构体模式）**，优雅地解决了多层级配置（如：程序默认值 -> 全局配置 -> 任务特有配置）的合并、提取以及**差分保存**问题。

## ✨ 核心特性

- 👯‍♂️ **自动生成 Partial 结构体**：只需要定义你业务所需的强类型配置对象，宏会自动为你生成一个所有字段均为 `Option<T>` 的影子配置类，用于解析和合并。
- 🚫 **业务逻辑零 `Option`**：调用 `.build()` 后，你将获得一个全量配置对象，彻底告别在业务代码中到处写 `.unwrap()`。
- ✂️ **差分保存 (Differential Saving)**：调用 `.simplify_from(parent)` 后，它能将子配置与父配置进行比对，自动剔除完全相同的字段。配合 `serde`，实现只记录覆盖项。
- 🪆 **支持深层嵌套**：通过 `#[config(nest)]` 属性，支持复杂配置的递归合并与递归化简。
- 🚀 **零开销的默认值推导**：支持常量的 `default = ...` 和按需惰性计算的表达式 `default_t = ...`。

## 📦 安装

在你的 `Cargo.toml` 中添加：

```toml
[dependencies]
inherit-config = "0.2.0"
serde = { version = "1.0", features = ["derive"] }
```

## 🚀 快速开始

### 1. 定义配置模型

```rust
use inherit_config::{ConfigLayer, InheritConfig};
use serde::{Deserialize, Serialize};

#[derive(InheritConfig, Clone, Debug, Serialize, Deserialize)]
pub struct DownloadConfig {
    // 基础字面量，当缺失时将回退到 32
    #[config(default = 32)]
    pub threads: usize,

    // 支持任意 Rust 表达式！采用延迟求值，避免不必要的内存分配
    #[config(default_t = String::from("system"))]
    pub proxy: String,
}

#[derive(InheritConfig, Clone, Debug, Serialize, Deserialize)]
pub struct Task {
    #[config(default_t = String::from("http://default.com"))]
    pub url: String,

    // 声明为嵌套配置，将递归处理合并与化简
    #[config(nest)]
    pub config: DownloadConfig,
}
```

### 2. 合并层级配置 (Inherit)

通常用于程序启动加载时：`任务配置 -> 继承全局配置 -> 缺失项使用默认值`。

```rust
// 1. 假设这是从 settings.toml 中反序列化出来的全局配置
let global_config = PartialTask {
    url: None,
    config: Some(PartialDownloadConfig {
        threads: Some(16),
        proxy: Some("global_proxy".to_string()),
    }),
};

// 2. 假设这是从 task.toml 中反序列化出来的具体任务配置
let mut task_config = PartialTask {
    url: Some("http://example.com/file.zip".to_string()),
    config: Some(PartialDownloadConfig {
        threads: Some(8), // 覆盖了线程数
        proxy: None,      // 没填代理，想要继承全局
    }),
};

// 3. 执行继承合并
task_config.inherit_from(&global_config);

// 4. 固化为全量业务对象 (No Options!)
let final_task: Task = task_config.build();

assert_eq!(final_task.config.threads, 8);               // 任务自身的值
assert_eq!(final_task.config.proxy, "global_proxy");    // 继承自全局
```

### 3. 神奇的差分保存 (Simplify)

当用户在 UI 或程序中修改了某个任务的配置，你想把它保存回 `task.toml`，但**只想保存那些与全局配置不一样的地方**。

```rust
let mut task_to_save = PartialTask {
    url: Some("http://example.com/file.zip".to_string()),
    config: Some(PartialDownloadConfig {
        threads: Some(16), // 用户改成了 16，但和全局配置一模一样！
        proxy: Some("local_proxy".to_string()), // 这个和全局不一样
    }),
};

// 执行化简：自动比对并剔除相同的字段
task_to_save.simplify_from(&global_config);

// 将精简后的对象序列化为 TOML
let toml_str = toml::to_string(&task_to_save).unwrap();
println!("{}", toml_str);
```

**输出的 TOML 极其干净，`threads = 16` 消失了！**

```toml
url = "http://example.com/file.zip"

[config]
proxy = "local_proxy"
```

## 🏷️ 属性指南

- `#[config(default = <literal>)]`
  用于简单的字面量或常量（如 `32`, `true`, `"str"`）。宏会将其转化为 `.unwrap_or(<literal>)` 以获得最佳性能。
- `#[config(default_t = <expression>)]`
  用于涉及内存分配或需要执行函数调用的复杂类型（如 `String::new()`, `vec![]`, `dirs::home_dir().unwrap()`）。宏会将其转化为 `.unwrap_or_else(|| <expression>)`，**实现真正的惰性求值**。
- `#[config(nest)]`
  用于标记嵌套配置结构体。宏会自动对该字段进行递归地 `inherit_from`、`simplify_from` 和 `build` 操作。注意被嵌套的结构体也必须 Derive 了 `InheritConfig`。
