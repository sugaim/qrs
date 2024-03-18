use std::{collections::HashMap, hash::Hash, ops::Deref};

use qrs_collections::MinSized;

// -----------------------------------------------------------------------------
// ComponentField
//
pub trait ComponentField {
    fn depends_on(&self) -> impl IntoIterator<Item = &str>;
}

impl ComponentField for f64 {
    #[inline]
    fn depends_on(&self) -> impl IntoIterator<Item = &str> {
        std::iter::empty()
    }
}

impl ComponentField for i64 {
    #[inline]
    fn depends_on(&self) -> impl IntoIterator<Item = &str> {
        std::iter::empty()
    }
}

impl ComponentField for bool {
    #[inline]
    fn depends_on(&self) -> impl IntoIterator<Item = &str> {
        std::iter::empty()
    }
}

impl ComponentField for String {
    #[inline]
    fn depends_on(&self) -> impl IntoIterator<Item = &str> {
        std::iter::once(self.as_str())
    }
}

impl<T> ComponentField for Option<T>
where
    T: ComponentField,
{
    #[inline]
    fn depends_on(&self) -> impl IntoIterator<Item = &str> {
        self.as_ref().map(|s| s.depends_on()).into_iter().flatten()
    }
}

impl<K, F> ComponentField for HashMap<K, F>
where
    K: Eq + Hash + ComponentField,
    F: ComponentField,
{
    #[inline]
    fn depends_on(&self) -> impl IntoIterator<Item = &str> {
        self.iter()
            .flat_map(|(k, v)| k.depends_on().into_iter().chain(v.depends_on()))
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
