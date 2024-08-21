use qfincore::{daycount::Act365f, Volatility};
use qmath::num::Real;

use crate::lnvol::LnCoord;

use super::super::{StrikeDer, VolCurve};

// -----------------------------------------------------------------------------
// Flat
// -----------------------------------------------------------------------------
#[derive(Debug, Clone, Copy, PartialEq, serde::Serialize, schemars::JsonSchema)]
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
impl<V: Real> VolCurve for Flat<V> {
    type Value = V;

    #[inline]
    fn bsvol(&self, _: &LnCoord<Self::Value>) -> anyhow::Result<Volatility<Act365f, Self::Value>> {
        Ok(Volatility {
            day_count: Act365f,
            value: self.vol.clone(),
        })
    }

    #[inline]
    fn bsvol_der(&self, coord: &LnCoord<V>) -> anyhow::Result<StrikeDer<V>> {
        self.bsvol(coord).map(|vol| StrikeDer {
            vol,
            dvdy: Volatility {
                day_count: Act365f,
                value: V::zero(),
            },
            d2vdy2: Volatility {
                day_count: Act365f,
                value: V::zero(),
            },
        })
    }
}
