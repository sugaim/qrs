use qmath::{ext::num::Zero, num::Scalar};

use crate::volsurf::slice::{LnCoord, LnVolSlice, StrikeDer};

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

impl<S: LnVolSlice> LnVolSlice for Weighted<S> {
    type Value = S::Value;

    #[inline]
    fn lnvol(&self, coord: &LnCoord<Self::Value>) -> anyhow::Result<Self::Value> {
        let mut sum = <S::Value as Zero>::zero();
        for (slice, weight) in &self.components {
            let weight = <S::Value as Scalar>::nearest_value_of_f64(*weight);
            let value = slice.lnvol(coord)?;
            sum += &(value * &weight);
        }
        Ok(sum)
    }

    #[inline]
    fn lnvol_der(
        &self,
        coord: &crate::volsurf::slice::LnCoord<Self::Value>,
    ) -> anyhow::Result<crate::volsurf::slice::StrikeDer<Self::Value>> {
        let mut sum = StrikeDer {
            vol: <S::Value as Zero>::zero(),
            dvdy: <S::Value as Zero>::zero(),
            d2vdy2: <S::Value as Zero>::zero(),
        };
        for (slice, weight) in &self.components {
            let weight = <S::Value as Scalar>::nearest_value_of_f64(*weight);
            let der = slice.lnvol_der(coord)?;
            sum.vol += &(der.vol * &weight);
            sum.dvdy += &(der.dvdy * &weight);
            sum.d2vdy2 += &(der.d2vdy2 * &weight);
        }
        Ok(sum)
    }
}
