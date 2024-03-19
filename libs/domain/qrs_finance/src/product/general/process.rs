mod constant;
mod deterministic;
mod market_ref;
mod ratio;

use anyhow::ensure;
use derivative::Derivative;
use qrs_finance_derive::Component;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::product::general::core::VariableTypes;

pub use constant::ConstantNumber;
pub use deterministic::DeterministicNumber;
pub use market_ref::MarketRef;
pub use ratio::Ratio;

// -----------------------------------------------------------------------------
// ValueType
//
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
pub enum ValueType {
    Number { dim: usize },
    Boolean { dim: usize },
    Integer { dim: usize },
}

// -----------------------------------------------------------------------------
// Process
//
#[derive(Derivative, Component, Serialize, Deserialize, JsonSchema)]
#[derivative(
    Debug(bound = "DeterministicNumber<Ts>: std::fmt::Debug,
        ConstantNumber<Ts>: std::fmt::Debug,
        MarketRef<Ts>: std::fmt::Debug,
        Ratio<Ts>: std::fmt::Debug"),
    Clone(bound = "DeterministicNumber<Ts>: Clone,
        ConstantNumber<Ts>: Clone,
        MarketRef<Ts>: Clone,
        Ratio<Ts>: Clone"),
    PartialEq(bound = "DeterministicNumber<Ts>: PartialEq,
        ConstantNumber<Ts>: PartialEq,
        MarketRef<Ts>: PartialEq,
        Ratio<Ts>: PartialEq")
)]
#[serde(
    tag = "type",
    rename_all = "snake_case",
    bound(
        serialize = "DeterministicNumber<Ts>: Serialize,
            ConstantNumber<Ts>: Serialize,
            MarketRef<Ts>: Serialize,
            Ratio<Ts>: Serialize",
        deserialize = "DeterministicNumber<Ts>: Deserialize<'de>,
            ConstantNumber<Ts>: Deserialize<'de>,
            MarketRef<Ts>: Deserialize<'de>,
            Ratio<Ts>: Deserialize<'de>"
    )
)]
#[schemars(bound = "Ts: JsonSchema,
        DeterministicNumber<Ts>: JsonSchema,
        ConstantNumber<Ts>: JsonSchema,
        MarketRef<Ts>: JsonSchema,
        Ratio<Ts>: JsonSchema")]
pub enum Process<Ts: VariableTypes> {
    DeterministicNumber(DeterministicNumber<Ts>),
    ConstantNumber(ConstantNumber<Ts>),
    MarketRef(MarketRef<Ts>),
    Ratio(Ratio<Ts>),
}

//
// methods
//
impl<Ts: VariableTypes> Process<Ts> {
    #[inline]
    pub fn value_type(&self) -> anyhow::Result<ValueType>
    where
        Ts::ProcessRef: AsRef<Self>,
    {
        let res = match self {
            Process::ConstantNumber(c) => ValueType::Number {
                dim: c.values.len(),
            },
            Process::DeterministicNumber(d) => ValueType::Number {
                dim: d.series.len(),
            },
            Process::MarketRef(m) => ValueType::Number { dim: m.refs.len() },
            Process::Ratio(r) => {
                let num = r.numer.value.as_ref().value_type()?;
                let denom = r.denom.value.as_ref().value_type()?;
                match (num, denom) {
                    (ValueType::Number { dim: n }, ValueType::Number { dim: d }) => {
                        ensure!(n == d, "Ratio must have the same dimension, but got {} for numerator and {} for denominator.", n, d);
                        ensure!(0 < n, "Ratio must have a positive dimension");
                        ValueType::Number { dim: n }
                    }
                    _ => return Err(anyhow::anyhow!("Ratio can only be applied to number type")),
                }
            }
        };
        Ok(res)
    }
}

// =============================================================================
#[cfg(test)]
mod tests {
    use std::collections::HashSet;

    use maplit::hashmap;
    use qrs_collections::RequireMinSize;
    use rstest::rstest;

    use crate::product::general::{
        core::{Component, HasDependency, ValueLess, ValueOrId, WithId},
        VariableTypesForData,
    };

    use super::*;

    fn constant() -> Process<VariableTypesForData> {
        Process::ConstantNumber(ConstantNumber {
            values: vec![
                ValueOrId::Id("cf1".into()),
                ValueOrId::Value(0.42),
                ValueOrId::Id("cf3".into()),
            ]
            .require_min_size()
            .unwrap(),
        })
    }

    fn deterministic() -> Process<VariableTypesForData> {
        Process::DeterministicNumber(DeterministicNumber {
            series: vec![
                hashmap! {
                    ValueOrId::Id("datetime1".into()) => ValueOrId::Value(0.42),
                    ValueOrId::Value("2024-01-01@tky".parse().unwrap()) =>ValueOrId::Id("num1".into()),
                }
                .require_min_size()
                .unwrap(),
                hashmap! {
                    ValueOrId::Value("2024-01-05@nyk".parse().unwrap()) =>ValueOrId::Id("num2".into()),
                    ValueOrId::Id("datetime2".into()) =>ValueOrId::Value(0.55),
                }
                .require_min_size()
                .unwrap(),
            ]
            .require_min_size()
            .unwrap(),
        })
    }

    fn market() -> Process<VariableTypesForData> {
        Process::MarketRef(MarketRef {
            refs: vec![
                WithId {
                    id: "mr1".into(),
                    value: ValueLess,
                },
                WithId {
                    id: "mr2".into(),
                    value: ValueLess,
                },
            ]
            .require_min_size()
            .unwrap(),
        })
    }

    fn ratio() -> Process<VariableTypesForData> {
        Process::Ratio(Ratio {
            numer: WithId {
                id: "numer".into(),
                value: ValueLess,
            },
            denom: WithId {
                id: "denom".into(),
                value: ValueLess,
            },
        })
    }

    #[rstest]
    #[case(constant())]
    #[case(deterministic())]
    #[case(market())]
    #[case(ratio())]
    fn test_category(#[case] proc: Process<VariableTypesForData>) {
        let cat = proc.category();

        assert_eq!(
            cat,
            crate::product::general::core::ComponentCategory::Process
        );
    }

    #[rstest]
    #[case(constant())]
    #[case(deterministic())]
    #[case(market())]
    #[case(ratio())]
    fn test_depends_on(#[case] proc: Process<VariableTypesForData>) {
        let expected: HashSet<_> = match &proc {
            Process::ConstantNumber(c) => c.depends_on().into_iter().collect(),
            Process::DeterministicNumber(d) => d.depends_on().into_iter().collect(),
            Process::MarketRef(m) => m.depends_on().into_iter().collect(),
            Process::Ratio(r) => r.depends_on().into_iter().collect(),
        };

        let deps: HashSet<_> = proc.depends_on().into_iter().collect();

        assert_eq!(deps, expected);
    }
}
