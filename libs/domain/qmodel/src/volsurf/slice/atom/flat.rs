use qmath::num::Real;

use super::super::{LnCoord, LnVolSlice, StrikeDer};

// -----------------------------------------------------------------------------
// Flat
// -----------------------------------------------------------------------------
#[derive(Debug, Clone, PartialEq, serde::Serialize, schemars::JsonSchema)]
pub struct Flat<V> {
    vol: V,
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
            vol: V,
        }
        let Inner { vol } = Inner::deserialize(deserializer)?;
        Self::new(vol).map_err(serde::de::Error::custom)
    }
}

//
// ctor
//
impl<V: Real> Flat<V> {
    /// Create a new flat volatility slice from a implied volatility in Act365f.
    ///
    /// Returns [`None`] if the implied volatility is negative.
    #[inline]
    pub fn new(vol: V) -> anyhow::Result<Self> {
        anyhow::ensure!(V::zero() <= vol, "implied volatility must be non-negative");
        Ok(Self { vol })
    }
}

//
// methods
//
impl<V: Real> LnVolSlice for Flat<V> {
    type Value = V;

    #[inline]
    fn lnvol(&self, _coord: &LnCoord<V>) -> anyhow::Result<V> {
        Ok(self.vol.clone())
    }

    #[inline]
    fn lnvol_der(&self, _coord: &LnCoord<V>) -> anyhow::Result<StrikeDer<V>> {
        Ok(StrikeDer {
            vol: self.vol.clone(),
            dvdy: V::zero(),
            d2vdy2: V::zero(),
        })
    }
}
