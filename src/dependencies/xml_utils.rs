use std::fmt::{Debug, Display, Formatter};

use serde::{Deserialize, Serialize};

#[derive(Clone, PartialEq, Default, Deserialize, Serialize)]
pub struct Elem<T> {
    #[serde(rename = "$value")]
    pub(crate) value: T,
}

impl<T> Elem<T> {
    pub fn new(value: T) -> Self {
        Self { value }
    }
}

// Surely there exist a more generic way to specify this

impl<T> From<T> for Elem<String>
where
    T: Into<String>,
{
    fn from(v: T) -> Self {
        Elem::new(v.into())
    }
}

impl<T> From<T> for Elem<bool>
where
    T: Into<bool>,
{
    fn from(v: T) -> Self {
        Elem::new(v.into())
    }
}

impl<E: Display> Display for Elem<E> {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        Display::fmt(&self.value, f)
    }
}

impl<E: Debug> Debug for Elem<E> {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        Debug::fmt(&self.value, f)
    }
}
