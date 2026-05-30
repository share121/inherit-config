#![no_std]

pub use inherit_config_derive::InheritConfig;

pub trait ConfigLayer {
    /// 对应的全量配置结构体类型
    type Full: Default;

    /// 继承合并：如果自己为空，则向 parent 借用
    fn inherit_from(&mut self, parent: &Self);

    /// 差分化简：移除与 parent 相同的字段，以及 parent 未设置时与默认值相同的字段
    fn simplify_from(&mut self, parent: &Self);

    /// 构建全量结构体：填补所有空白字段为默认值
    fn build(self) -> Self::Full;

    /// 从全量结构体构造 Partial：所有字段包装为 Some
    fn from_full(full: Self::Full) -> Self;
}
