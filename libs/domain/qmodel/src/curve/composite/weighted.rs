use qchrono::timepoint::DateTime;
use qfincore::{daycount::Act365f, Yield};
use qmath::num::Scalar;

use super::super::YieldCurve;

// -----------------------------------------------------------------------------
// Weighted
// -----------------------------------------------------------------------------
/// A weighted curve.
///
/// Forward rate of this curve is calculated by the weighted average of the forward rates of
/// the component curves.
/// For example, if there are two components `A` and `B` with weights `wA` and `wB` respectively,
/// the forward rate of this curve is calculated as `wA * A.forward_rate(from, to) + wB * B.forward_rate(from, to)`.
#[derive(Debug, Clone, PartialEq)]
pub struct Weighted<C> {
    pub components: Vec<(C, f64)>,
}

//
// ser/de
//
impl<C: serde::Serialize> serde::Serialize for Weighted<C> {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        #[derive(serde::Serialize)]
        struct Component<'a, C> {
            curve: &'a C,
            weight: f64,
        }
        #[derive(serde::Serialize)]
        struct Components<'a, C> {
            components: Vec<Component<'a, C>>,
        }
        Components {
            components: self
                .components
                .iter()
                .map(|(curve, weight)| Component {
                    curve,
                    weight: *weight,
                })
                .collect(),
        }
        .serialize(serializer)
    }
}

impl<'de, C> serde::Deserialize<'de> for Weighted<C>
where
    C: serde::Deserialize<'de>,
{
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        #[derive(serde::Deserialize)]
        struct Component<C> {
            curve: C,
            weight: f64,
        }
        #[derive(serde::Deserialize)]
        struct Components<C> {
            components: Vec<Component<C>>,
        }
        let Components { components } = Components::deserialize(deserializer)?;
        Ok(Weighted {
            components: components
                .into_iter()
                .map(|c| (c.curve, c.weight))
                .collect(),
        })
    }
}

impl<C: schemars::JsonSchema> schemars::JsonSchema for Weighted<C> {
    fn schema_name() -> String {
        format!("Weighted_for_{}", C::schema_name())
    }
    fn schema_id() -> std::borrow::Cow<'static, str> {
        format!("qmodel::curve::composite::Weighted<{}>", C::schema_id()).into()
    }

    fn json_schema(gen: &mut schemars::gen::SchemaGenerator) -> schemars::schema::Schema {
        #[derive(schemars::JsonSchema)]
        #[allow(dead_code)]
        struct Component<C> {
            curve: C,
            weight: f64,
        }
        #[derive(schemars::JsonSchema)]
        #[allow(dead_code)]
        struct Components<C> {
            components: Vec<Component<C>>,
        }
        Components::<C>::json_schema(gen)
    }
}

//
// methods
//
impl<C> YieldCurve for Weighted<C>
where
    C: YieldCurve,
{
    type Value = C::Value;

    fn forward_rate(
        &self,
        from: &DateTime,
        to: &DateTime,
    ) -> anyhow::Result<Yield<Act365f, Self::Value>> {
        let mut total_contrib = <C::Value as qmath::ext::num::Zero>::zero();
        for (curve, weight) in &self.components {
            let contrib = curve.forward_rate(from, to)?.value;
            total_contrib += &(contrib * &<C::Value as Scalar>::nearest_value_of_f64(*weight));
        }
        Ok(total_contrib.into())
    }
}

#[cfg(test)]
mod tests {
    use rstest::rstest;

    use crate::curve::atom::Flat;

    use super::*;

    #[rstest]
    #[case(vec![], 0.)]
    #[case(vec![1.], 0.01)]
    #[case(vec![-1.], -0.01)]
    #[case(vec![1., 1.], 0.03)]
    #[case(vec![1., 1., -1.], 0.0)]
    fn test_forward_rate(#[case] weights: Vec<f64>, #[case] expected: f64) {
        let curve = Weighted {
            components: weights
                .into_iter()
                .enumerate()
                .map(|(i, weight)| {
                    (
                        Flat {
                            rate: 0.01 * (i + 1) as f64,
                        },
                        weight,
                    )
                })
                .collect(),
        };

        let res = curve
            .forward_rate(
                &"2021-01-01T00:00:00Z".parse().unwrap(),
                &"2021-01-02T00:00:00Z".parse().unwrap(),
            )
            .unwrap()
            .value;

        approx::assert_abs_diff_eq!(res, expected, epsilon = 1e-10);
    }
}
