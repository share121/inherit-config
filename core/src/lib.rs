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
