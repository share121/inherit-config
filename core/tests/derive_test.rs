#![cfg(feature = "derive")]

use inherit_config::{Config, ConfigField, InheritAble};

#[derive(Clone, Config)]
struct TestConfig {
    #[config(default = ConfigField::Unset)]
    field1: ConfigField<String>,
    #[config(default = Some(42))]
    field2: Option<u32>,
    #[config(default = 100, skip_inherit, skip_simplify)]
    field3: i32,
}

#[test]
fn test_derive_macro() {
    let default_config = TestConfig::default();
    assert!(matches!(default_config.field1, ConfigField::Unset));
    assert_eq!(default_config.field2, Some(42));
    assert_eq!(default_config.field3, 100);

    let child = TestConfig {
        field1: ConfigField::Inherit,
        field2: None,
        field3: 200,
    };
    let parent = TestConfig {
        field1: ConfigField::Set("hello".to_string()),
        field2: Some(99),
        field3: 300,
    };
    let inherited = child.inherit(&parent);
    // field1 inherits from parent because child is Inherit
    assert!(matches!(inherited.field1, ConfigField::Set(ref s) if s == "hello"));
    // field2 is None, inherits from parent Some(99)
    assert_eq!(inherited.field2, Some(99));
    // field3 skips inherit, remains child's value
    assert_eq!(inherited.field3, 200);
    // 使用 get 方法
    assert_eq!(child.field2(), 42);
}
