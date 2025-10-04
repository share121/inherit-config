#![no_std]
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub enum ConfigField<T> {
    #[default]
    Inherit,
    Unset,
    Set(T),
}

impl<T> ConfigField<T> {
    pub fn get(&self) -> Option<&T> {
        match self {
            ConfigField::Set(t) => Some(t),
            _ => None,
        }
    }
}

pub trait InheritAble: Clone {
    fn inherit(&self, _other: &Self) -> Self {
        self.clone()
    }
}

impl<T: Clone> InheritAble for ConfigField<T> {
    fn inherit(&self, other: &Self) -> Self {
        match self {
            ConfigField::Inherit => other,
            _ => self,
        }
        .clone()
    }
}

impl<T: Clone> InheritAble for Option<T> {
    fn inherit(&self, other: &Self) -> Self {
        match self {
            None => other,
            _ => self,
        }
        .clone()
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
    fn test_config_field_default() {
        let default_field: ConfigField<i32> = ConfigField::default();
        assert_eq!(default_field, ConfigField::Inherit);
    }

    #[test]
    fn test_config_field_inherit() {
        let parent_set = ConfigField::Set(100);
        let parent_unset = ConfigField::Unset;
        let parent_inherit: ConfigField<i32> = ConfigField::Inherit;

        let child_inherit: ConfigField<i32> = ConfigField::Inherit;
        let child_set = ConfigField::Set(200);
        let child_unset = ConfigField::Unset;

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
        let parent_none: Option<i32> = None;

        let child_some = Some(200);
        let child_none: Option<i32> = None;

        // 当子级为 None 时，它应该从父级继承
        assert_eq!(child_none.inherit(&parent_some), parent_some);
        assert_eq!(child_none.inherit(&parent_none), parent_none);

        // 当子级为 Some 时，它应该保持自己的值
        assert_eq!(child_some.inherit(&parent_some), child_some);
        assert_eq!(child_some.inherit(&parent_none), child_some);
    }
}
