use inherit_config::{ConfigLayer, InheritConfig};

#[derive(InheritConfig, Clone, Debug)]
pub struct RetryConfig {
    #[config(default = 3)]
    pub max_retries: usize,

    #[config(default_t = String::from("exponential"))]
    pub strategy: String,
}

#[derive(InheritConfig, Clone, Debug)]
pub struct DownloadConfig {
    #[config(default = 32)]
    pub threads: usize,

    #[config(default_t = String::from("system"))]
    pub proxy: String,

    #[config(nest)]
    pub retry: RetryConfig,
}

#[derive(InheritConfig, Clone, Debug)]
pub struct Task {
    #[config(default_t = String::from("http://default.com"))]
    pub url: String,

    #[config(nest)]
    pub config: DownloadConfig,
}

#[test]
fn test_default_build() {
    // 创建一个完全为空的 Partial 对象
    let empty_partial = PartialTask::default();

    // 固化为全量对象
    let full_task = empty_partial.build();

    // 验证：是否正确应用了默认值和 default_with 表达式
    assert_eq!(full_task.url, "http://default.com");
    assert_eq!(full_task.config.threads, 32);
    assert_eq!(full_task.config.proxy, "system");
    assert_eq!(full_task.config.retry.max_retries, 3);
    assert_eq!(full_task.config.retry.strategy, "exponential");
}

#[test]
fn test_inherit_logic() {
    // 模拟全局配置（父节点）
    let parent = PartialDownloadConfig {
        threads: Some(16),
        proxy: Some("parent_proxy".to_string()),
        retry: Some(PartialRetryConfig {
            max_retries: Some(5),
            strategy: None, // 留空，测试是否回退到 default
        }),
    };

    // 模拟任务配置（子节点）
    let mut child = PartialDownloadConfig {
        threads: Some(8), // 覆盖父节点
        proxy: None,      // 留空，应该继承父节点
        retry: Some(PartialRetryConfig {
            max_retries: None,                    // 留空，继承父节点
            strategy: Some("linear".to_string()), // 覆盖
        }),
    };

    // 执行继承合并
    child.inherit_from(&parent);

    // 验证合并后的 Partial 状态
    assert_eq!(child.threads, Some(8));
    assert_eq!(child.proxy, Some("parent_proxy".to_string()));

    let child_retry = child.retry.as_ref().unwrap();
    assert_eq!(child_retry.max_retries, Some(5));
    assert_eq!(child_retry.strategy, Some("linear".to_string()));

    // 验证 Build 后的最终结果
    let full = child.build();
    assert_eq!(full.threads, 8);
    assert_eq!(full.proxy, "parent_proxy");
    assert_eq!(full.retry.max_retries, 5);
    // 这里验证了 None 在 build 时触发默认值逻辑
    assert_eq!(full.retry.strategy, "linear");
}

#[test]
fn test_simplify_logic_for_diff_saving() {
    // 模拟系统全局配置
    let parent = PartialTask {
        url: None,
        config: Some(PartialDownloadConfig {
            threads: Some(100),
            proxy: Some("global_proxy".to_string()),
            retry: Some(PartialRetryConfig {
                max_retries: Some(10),
                strategy: Some("fixed".to_string()),
            }),
        }),
    };

    // 模拟在 UI 或代码中被修改过的具体任务配置
    let mut child = PartialTask {
        url: Some("http://example.com/file.zip".to_string()),
        config: Some(PartialDownloadConfig {
            threads: Some(100),                     // 与父节点相同，应该被化简掉 (None)
            proxy: Some("local_proxy".to_string()), // 与父节点不同，保留
            retry: Some(PartialRetryConfig {
                max_retries: Some(10),               // 与父节点相同 -> None
                strategy: Some("fixed".to_string()), // 与父节点相同 -> None
            }),
        }),
    };

    // 执行化简（核心功能：差分保存）
    child.simplify_from(&parent);

    // 验证：相同的字段全部变成了 None
    assert_eq!(child.url, Some("http://example.com/file.zip".to_string()));

    let child_config = child.config.unwrap();
    assert_eq!(child_config.threads, None); // 被成功化简
    assert_eq!(child_config.proxy, Some("local_proxy".to_string())); // 保留了差异

    assert_eq!(child_config.retry, None); // 被成功化简
}

#[test]
fn test_simplify_default_value() {
    // parent 没设 max_retries (None → 走默认值 3)，child 显式设了 3（和默认值一样）
    let parent = PartialRetryConfig {
        max_retries: None,
        strategy: None,
    };

    let mut child = PartialRetryConfig {
        max_retries: Some(3),
        strategy: Some(String::from("exponential")),
    };

    child.simplify_from(&parent);

    // max_retries parent=None, child=Some(3)==default(3) → 化简为 None
    assert_eq!(child.max_retries, None);
    // strategy parent=None, child=Some("exponential")==default_t(String::from("exponential")) → 化简为 None
    assert_eq!(child.strategy, None);
}

#[test]
fn test_simplify_nest_default() {
    // parent 没设 retry (None → 走默认值)，child 显式设了所有字段且都等于默认值
    let parent = PartialDownloadConfig {
        threads: Some(100),
        proxy: Some("global_proxy".to_string()),
        retry: None,
    };

    let mut child = PartialDownloadConfig {
        threads: Some(100),                     // 和 parent 相同 → None
        proxy: Some("local_proxy".to_string()), // 和 parent 不同 → 保留
        retry: Some(PartialRetryConfig {
            max_retries: Some(3),                        // 默认值 3 → None
            strategy: Some(String::from("exponential")), // 默认值 → None
        }),
    };

    child.simplify_from(&parent);

    assert_eq!(child.threads, None);
    assert_eq!(child.proxy, Some("local_proxy".to_string()));
    assert_eq!(child.retry, None); // 全部是默认值 → 整个化简掉
}

#[test]
fn test_from_full() {
    // 构造一个 Full Config
    let full = Task {
        url: "http://example.com".to_string(),
        config: DownloadConfig {
            threads: 16,
            proxy: "custom_proxy".to_string(),
            retry: RetryConfig {
                max_retries: 5,
                strategy: "linear".to_string(),
            },
        },
    };

    // 方式一：通过 trait 方法转换
    let partial = PartialTask::from_full(full.clone());
    assert_eq!(partial.url, Some("http://example.com".to_string()));
    let cfg = partial.config.as_ref().unwrap();
    assert_eq!(cfg.threads, Some(16));
    assert_eq!(cfg.proxy.as_ref(), Some(&"custom_proxy".to_string()));
    let retry = cfg.retry.as_ref().unwrap();
    assert_eq!(retry.max_retries, Some(5));
    assert_eq!(retry.strategy.as_ref(), Some(&"linear".to_string()));

    // build 回去应该得到相同的 Full
    let roundtrip = partial.clone().build();
    assert_eq!(roundtrip.url, full.url);
    assert_eq!(roundtrip.config.threads, full.config.threads);
    assert_eq!(roundtrip.config.proxy, full.config.proxy);
    assert_eq!(
        roundtrip.config.retry.max_retries,
        full.config.retry.max_retries
    );
    assert_eq!(roundtrip.config.retry.strategy, full.config.retry.strategy);

    // 方式二：通过 From/Into trait
    let partial2: PartialTask = full.into();
    assert_eq!(partial2.url, Some("http://example.com".to_string()));
}
