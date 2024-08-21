use qfincore::{daycount::Act365f, Volatility};
use qmath::{ext::num::Zero, num::Scalar};

use crate::lnvol::{
    curve::{StrikeDer, VolCurve},
    LnCoord,
};

// -----------------------------------------------------------------------------
// Weighted
// -----------------------------------------------------------------------------
#[derive(Clone, Debug, PartialEq)]
pub struct Weighted<S> {
    pub components: Vec<(S, f64)>,
}

impl<S: serde::Serialize> serde::Serialize for Weighted<S> {
    fn serialize<Se>(&self, serializer: Se) -> Result<Se::Ok, Se::Error>
    where
        Se: serde::Serializer,
    {
        #[derive(serde::Serialize)]
        struct Component<'a, S> {
            slice: &'a S,
            weight: f64,
        }
        #[derive(serde::Serialize)]
        struct Components<'a, S> {
            components: Vec<Component<'a, S>>,
        }
        Components {
            components: self
                .components
                .iter()
                .map(|(slice, weight)| Component {
                    slice,
                    weight: *weight,
                })
                .collect(),
        }
        .serialize(serializer)
    }
}

impl<'de, S> serde::Deserialize<'de> for Weighted<S>
where
    S: serde::Deserialize<'de>,
{
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        #[derive(serde::Deserialize)]
        struct Component<S> {
            slice: S,
            weight: f64,
        }
        #[derive(serde::Deserialize)]
        struct Components<S> {
            components: Vec<Component<S>>,
        }
        let Components { components } = Components::deserialize(deserializer)?;
        Ok(Weighted {
            components: components
                .into_iter()
                .map(|c| (c.slice, c.weight))
                .collect(),
        })
    }
}

impl<S: VolCurve> VolCurve for Weighted<S> {
    type Value = S::Value;

    #[inline]
    fn bsvol(
        &self,
        coord: &LnCoord<Self::Value>,
    ) -> anyhow::Result<Volatility<Act365f, Self::Value>> {
        let mut sum = <S::Value as Zero>::zero();
        for (slice, weight) in &self.components {
            let weight = <S::Value as Scalar>::nearest_value_of_f64(*weight);
            let value = slice.bsvol(coord)?.value;
            sum += &(value * &weight);
        }
        Ok(qfincore::Volatility {
            day_count: qfincore::daycount::Act365f,
            value: sum,
        })
    }

    #[inline]
    fn bsvol_der(&self, coord: &LnCoord<Self::Value>) -> anyhow::Result<StrikeDer<Self::Value>> {
        let mut sum = {
            let zero = Volatility {
                day_count: Act365f,
                value: <Self::Value as Zero>::zero(),
            };
            StrikeDer {
                vol: zero.clone(),
                dvdy: zero.clone(),
                d2vdy2: zero,
            }
        };
        for (slice, weight) in &self.components {
            let weight = <S::Value as Scalar>::nearest_value_of_f64(*weight);
            let der = slice.bsvol_der(coord)?;
            sum.vol.value += &(der.vol.value * &weight);
            sum.dvdy.value += &(der.dvdy.value * &weight);
            sum.d2vdy2.value += &(der.d2vdy2.value * &weight);
        }
        Ok(sum)
    }
}
