#![no_std]

#[cfg(feature = "derive")]
pub use inherit_config_derive::Config;

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub enum ConfigField<T> {
    #[default]
    Inherit,
    Unset,
    Set(T),
}

impl<T> ConfigField<T> {
    pub const fn get(&self) -> Option<&T> {
        match self {
            Self::Set(t) => Some(t),
            _ => None,
        }
    }
}

pub trait InheritAble {
    #[must_use]
    fn inherit(&self, other: &Self) -> Self;
    fn simplify(&mut self, other: &Self);
}

impl<T: Clone + PartialEq> InheritAble for ConfigField<T> {
    fn inherit(&self, other: &Self) -> Self {
        match self {
            &Self::Inherit => other,
            _ => self,
        }
        .clone()
    }
    fn simplify(&mut self, other: &Self) {
        if self == other {
            *self = Self::Inherit;
        }
    }
}

impl<T: Clone + PartialEq> InheritAble for Option<T> {
    fn inherit(&self, other: &Self) -> Self {
        match self {
            None => other,
            _ => self,
        }
        .clone()
    }
    fn simplify(&mut self, other: &Self) {
        if self == other {
            *self = None;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_config_field_get() {
        let field_set = ConfigField::Set(42);
        assert_eq!(field_set.get(), Some(&42));

        let field_inherit: ConfigField<i32> = ConfigField::Inherit;
        assert_eq!(field_inherit.get(), None);

        let field_unset: ConfigField<i32> = ConfigField::Unset;
        assert_eq!(field_unset.get(), None);
    }

    #[test]
    fn test_config_field_inherit() {
        let parent_set = ConfigField::Set(100);
        let parent_unset = ConfigField::Unset;
        let parent_inherit = ConfigField::Inherit;

        let child_set = ConfigField::Set(200);
        let child_unset = ConfigField::Unset;
        let child_inherit = ConfigField::Inherit;

        // 当子级为 Inherit 时，它应该从父级继承
        assert_eq!(child_inherit.inherit(&parent_set), parent_set);
        assert_eq!(child_inherit.inherit(&parent_unset), parent_unset);
        assert_eq!(child_inherit.inherit(&parent_inherit), parent_inherit);

        // 当子级为 Set 时，它应该保持自己的值
        assert_eq!(child_set.inherit(&parent_set), child_set);
        assert_eq!(child_set.inherit(&parent_unset), child_set);
        assert_eq!(child_set.inherit(&parent_inherit), child_set);

        // 当子级为 Unset 时，它也应该保持自己的状态
        assert_eq!(child_unset.inherit(&parent_set), child_unset);
        assert_eq!(child_unset.inherit(&parent_unset), child_unset);
        assert_eq!(child_unset.inherit(&parent_inherit), child_unset);
    }

    #[test]
    fn test_option_inherit() {
        let parent_some = Some(100);
        let parent_none = None;

        let child_some = Some(200);
        let child_none = None;

        // 当子级为 None 时，它应该从父级继承
        assert_eq!(child_none.inherit(&parent_some), parent_some);
        assert_eq!(child_none.inherit(&parent_none), parent_none);

        // 当子级为 Some 时，它应该保持自己的值
        assert_eq!(child_some.inherit(&parent_some), child_some);
        assert_eq!(child_some.inherit(&parent_none), child_some);
    }

    #[test]
    fn test_simplify_logic() {
        let parent = ConfigField::Set(100);
        let mut child = ConfigField::Set(100); // 子级和父级值一样

        child.simplify(&parent);

        assert_eq!(child, ConfigField::Inherit);
        assert_eq!(child.inherit(&parent), ConfigField::Set(100));
    }

    #[test]
    fn test_simplify_option() {
        let parent = Some(50);
        let mut child = Some(50);

        child.simplify(&parent);

        assert_eq!(child, None);
        assert_eq!(child.inherit(&parent), Some(50));
    }
}
