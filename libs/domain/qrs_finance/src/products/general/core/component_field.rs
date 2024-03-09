use std::{collections::HashMap, hash::Hash, ops::Deref};

use qrs_collections::MinSized;

// -----------------------------------------------------------------------------
// ComponentField
//
pub trait ComponentField {
    fn depends_on(&self) -> impl IntoIterator<Item = &str>;
}

impl ComponentField for String {
    #[inline]
    fn depends_on(&self) -> impl IntoIterator<Item = &str> {
        std::iter::once(self.as_str())
    }
}

impl<K, F> ComponentField for HashMap<K, F>
where
    K: Eq + Hash,
    F: ComponentField,
{
    #[inline]
    fn depends_on(&self) -> impl IntoIterator<Item = &str> {
        self.values().flat_map(ComponentField::depends_on)
    }
}

impl<F> ComponentField for Vec<F>
where
    F: ComponentField,
{
    #[inline]
    fn depends_on(&self) -> impl IntoIterator<Item = &str> {
        self.iter().flat_map(ComponentField::depends_on)
    }
}

impl<F, const N: usize> ComponentField for MinSized<F, N>
where
    F: ComponentField,
{
    #[inline]
    fn depends_on(&self) -> impl IntoIterator<Item = &str> {
        self.deref().depends_on()
    }
}
