use std::{borrow::Borrow, hash::Hash};

use num::Zero;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

// -----------------------------------------------------------------------------
// Positive
//
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, JsonSchema)]
pub struct Positive<V>(V);

impl<V> Positive<V> {
    #[inline]
    pub fn new(value: V) -> Option<Self>
    where
        V: PartialOrd + Zero,
    {
        if value > V::zero() {
            Some(Positive(value))
        } else {
            None
        }
    }
}

impl<V> Positive<V> {
    #[inline]
    pub fn into_inner(self) -> V {
        self.0
    }
}

impl<V> AsRef<V> for Positive<V> {
    #[inline]
    fn as_ref(&self) -> &V {
        &self.0
    }
}

impl<V> Borrow<V> for Positive<V> {
    #[inline]
    fn borrow(&self) -> &V {
        &self.0
    }
}

impl<V> Hash for Positive<V>
where
    V: Hash,
{
    #[inline]
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.0.hash(state)
    }
}

impl<'de, V> Deserialize<'de> for Positive<V>
where
    V: Deserialize<'de> + PartialOrd + Zero,
{
    fn deserialize<D: serde::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        Self::new(V::deserialize(deserializer)?)
            .ok_or_else(|| serde::de::Error::custom("Positive value must be greater than zero"))
    }
}
