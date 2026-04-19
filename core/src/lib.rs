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

impl<T: Clone + Default + PartialEq> InheritAble for T {
    fn inherit(&self, other: &Self) -> Self {
        if self == &Self::default() {
            other
        } else {
            self
        }
        .clone()
    }
    fn simplify(&mut self, other: &Self) {
        if self == other {
            *self = Self::default();
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
    fn test_string_inherit() {
        let parent_a = "parent_a";
        let parent_b = "";

        let child_a = "child_a";
        let child_b = "";

        // 当子级为 "" 时，它应该从父级继承
        assert_eq!(child_b.inherit(&parent_a), parent_a);
        assert_eq!(child_b.inherit(&parent_b), parent_b);

        // 当子级不为 "" 时，它应该保持自己的值
        assert_eq!(child_a.inherit(&parent_a), child_a);
        assert_eq!(child_a.inherit(&parent_b), child_a);
    }

    #[test]
    fn test_bool_inherit() {
        let parent_true = true;
        let parent_false = false;

        let child_true = true;
        let child_false = false;

        // 当子级为 false 时，它应该从父级继承
        assert_eq!(child_false.inherit(&parent_true), parent_true);
        assert_eq!(child_false.inherit(&parent_false), parent_false);

        // 当子级为 true 时，它应该保持自己的值
        assert_eq!(child_true.inherit(&parent_true), child_true);
        assert_eq!(child_true.inherit(&parent_false), child_true);
    }

    #[test]
    fn test_number_inherit() {
        let parent_100 = 100;
        let parent_0 = 0;

        let child_200 = 200;
        let child_0 = 0;

        // 当子级为 0 时，它应该从父级继承
        assert_eq!(child_0.inherit(&parent_100), parent_100);
        assert_eq!(child_0.inherit(&parent_0), parent_0);

        // 当子级不为 0 时，它应该保持自己的值
        assert_eq!(child_200.inherit(&parent_100), child_200);
        assert_eq!(child_200.inherit(&parent_0), child_200);
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

    #[test]
    fn test_simplify_str() {
        let parent = "hello";
        let mut child = "hello";

        child.simplify(&parent);

        assert_eq!(child, "");
        assert_eq!(child.inherit(&parent), "hello");
    }

    #[test]
    #[allow(clippy::bool_assert_comparison)]
    fn test_simplify_bool() {
        let parent = true;
        let mut child = true;

        child.simplify(&parent);

        assert_eq!(child, false);
        assert_eq!(child.inherit(&parent), true);
    }

    #[test]
    fn test_simplify_number() {
        let parent = 100;
        let mut child = 100;

        child.simplify(&parent);

        assert_eq!(child, 0);
        assert_eq!(child.inherit(&parent), 100);
    }
}
