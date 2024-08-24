use anyhow::ensure;
use qfincore::{daycount::Act365f, Volatility};
use qmath::num::Real;

use crate::lnvol::{
    curve::{StrikeDer, VolCurve},
    LnCoord,
};

// -----------------------------------------------------------------------------
// Scaled
// -----------------------------------------------------------------------------
#[derive(Clone, Debug, PartialEq, serde::Serialize, schemars::JsonSchema)]
pub struct Scaled<S, V> {
    base: S,
    scale: V,
}

//
// serde
//
impl<'de, S, V> serde::Deserialize<'de> for Scaled<S, V>
where
    S: serde::Deserialize<'de>,
    V: serde::Deserialize<'de> + Real,
{
    #[inline]
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        #[derive(serde::Deserialize)]
        struct Inner<S, V> {
            base: S,
            scale: V,
        }
        let Inner { base, scale } = Inner::deserialize(deserializer)?;
        Self::new(base, scale).map_err(serde::de::Error::custom)
    }
}

//
// ctor
//
impl<S, V> Scaled<S, V> {
    #[inline]
    pub fn new(base: S, scale: V) -> anyhow::Result<Self>
    where
        V: Real,
    {
        ensure!(V::zero() < scale, "scaling factor must be positive");
        Ok(Self { base, scale })
    }
}

impl<S, V> VolCurve for Scaled<S, V>
where
    S: VolCurve,
    V: Into<S::Value> + Clone,
{
    type Value = S::Value;

    #[inline]
    fn bsvol(
        &self,
        coord: &LnCoord<Self::Value>,
    ) -> anyhow::Result<Volatility<Act365f, Self::Value>> {
        let vol = self.base.bsvol(coord)?;
        Ok(Volatility {
            day_count: Act365f,
            value: vol.value * &self.scale.clone().into(),
        })
    }

    #[inline]
    fn bsvol_der(&self, coord: &LnCoord<Self::Value>) -> anyhow::Result<StrikeDer<Self::Value>> {
        let der = self.base.bsvol_der(coord)?;
        let mult = self.scale.clone().into();
        Ok(StrikeDer {
            vol: Volatility {
                day_count: Act365f,
                value: der.vol.value * &mult,
            },
            dvdy: Volatility {
                day_count: Act365f,
                value: der.dvdy.value * &mult,
            },
            d2vdy2: Volatility {
                day_count: Act365f,
                value: der.d2vdy2.value * &mult,
            },
        })
    }
}
