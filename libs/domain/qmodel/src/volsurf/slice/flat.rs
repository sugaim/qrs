use qmath::num::Real;

use super::{LnCoord, LnVolSlice, VarianceStrikeDer};

// -----------------------------------------------------------------------------
// Flat
// -----------------------------------------------------------------------------
#[derive(Debug, Clone, PartialEq, serde::Serialize, schemars::JsonSchema)]
pub struct Flat<V> {
    iv: V,
}

//
// ser/de
//
impl<'de, V: Real + serde::Deserialize<'de>> serde::Deserialize<'de> for Flat<V> {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        #[derive(serde::Deserialize)]
        struct Inner<V> {
            iv: V,
        }
        let Inner { iv } = Inner::deserialize(deserializer)?;
        Self::new(iv)
            .ok_or_else(|| serde::de::Error::custom("implied volatility must be non-negative"))
    }
}

//
// ctor
//
impl<V: Real> Flat<V> {
    /// Create a new flat volatility slice from a implied volatility in Act365f.
    ///
    /// Returns [`None`] if the implied volatility is negative.
    pub fn new(iv: V) -> Option<Self> {
        if iv < V::zero() {
            None
        } else {
            Some(Self { iv })
        }
    }
}

//
// methods
//
impl<V: Real> Flat<V> {
    /// Get the implied volatility.
    #[inline]
    pub fn iv(&self) -> &V {
        &self.iv
    }
}

impl<V: Real> LnVolSlice for Flat<V> {
    type Value = V;

    #[inline]
    fn iv(&self, _coord: &LnCoord<V>) -> anyhow::Result<V> {
        Ok(self.iv.clone())
    }

    #[inline]
    fn variance_der(&self, _coord: &LnCoord<V>) -> anyhow::Result<VarianceStrikeDer<V>> {
        Ok(VarianceStrikeDer {
            var: self.iv.clone(),
            dvdy: V::zero(),
            d2vdy2: V::zero(),
        })
    }
}
