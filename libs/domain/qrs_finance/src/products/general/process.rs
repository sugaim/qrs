mod constant;
mod deterministic;
mod market_ref;

use derivative::Derivative;
use qrs_finance_derive::Component;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::products::general::core::VariableTypes;

pub use constant::ConstantFloat;
pub use deterministic::DeterministicFloat;
pub use market_ref::MarketRef;

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
    Debug(bound = "DeterministicFloat<Ts>: std::fmt::Debug,
        ConstantFloat<Ts>: std::fmt::Debug,
        MarketRef<Ts>: std::fmt::Debug"),
    Clone(bound = "DeterministicFloat<Ts>: Clone,
        ConstantFloat<Ts>: Clone,
        MarketRef<Ts>: Clone"),
    PartialEq(bound = "DeterministicFloat<Ts>: PartialEq,
        ConstantFloat<Ts>: PartialEq,
        MarketRef<Ts>: PartialEq")
)]
#[serde(
    tag = "type",
    rename_all = "snake_case",
    bound(
        serialize = "DeterministicFloat<Ts>: Serialize,
            ConstantFloat<Ts>: Serialize,
            MarketRef<Ts>: Serialize",
        deserialize = "DeterministicFloat<Ts>: Deserialize<'de>,
            ConstantFloat<Ts>: Deserialize<'de>,
            MarketRef<Ts>: Deserialize<'de>"
    )
)]
#[schemars(bound = "Ts: JsonSchema,
        DeterministicFloat<Ts>: JsonSchema,
        ConstantFloat<Ts>: JsonSchema,
        MarketRef<Ts>: JsonSchema")]
pub enum Process<Ts: VariableTypes> {
    DeterministicFloat(DeterministicFloat<Ts>),
    ConstantFloat(ConstantFloat<Ts>),
    MarketRef(MarketRef<Ts>),
}

//
// methods
//
impl<Ts: VariableTypes> Process<Ts> {
    #[inline]
    pub fn value_type(&self) -> ValueType
    where
        Ts::ProcessRef: AsRef<Self>,
    {
        match self {
            Process::ConstantFloat(c) => ValueType::Number {
                dim: c.values.len(),
            },
            Process::DeterministicFloat(d) => ValueType::Number {
                dim: d.series.len(),
            },
            Process::MarketRef(m) => ValueType::Number { dim: m.refs.len() },
        }
    }
}

// =============================================================================
#[cfg(test)]
mod tests {
    use std::collections::HashSet;

    use maplit::hashmap;
    use qrs_collections::RequireMinSize;
    use rstest::rstest;

    use crate::products::general::{
        core::{Component, ValueLess, ValueOrId, WithId},
        VariableTypesForData,
    };

    use super::*;

    fn constant() -> Process<VariableTypesForData> {
        Process::ConstantFloat(ConstantFloat {
            values: vec![
                ValueOrId::Id("cf1".to_string()),
                ValueOrId::Value(0.42),
                ValueOrId::Id("cf3".to_string()),
            ]
            .require_min_size()
            .unwrap(),
        })
    }

    fn deterministic() -> Process<VariableTypesForData> {
        Process::DeterministicFloat(DeterministicFloat {
            series: vec![
                hashmap! {
                    ValueOrId::Id("datetime1".to_string()) => ValueOrId::Value(0.42),
                    ValueOrId::Value("2024-01-01@tky".parse().unwrap()) =>ValueOrId::Id("num1".to_string()),
                }
                .require_min_size()
                .unwrap(),
                hashmap! {
                    ValueOrId::Value("2024-01-05@nyk".parse().unwrap()) =>ValueOrId::Id("num2".to_string()),
                    ValueOrId::Id("datetime2".to_string()) =>ValueOrId::Value(0.55),
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
                    id: "mr1".to_string(),
                    value: ValueLess,
                },
                WithId {
                    id: "mr2".to_string(),
                    value: ValueLess,
                },
            ]
            .require_min_size()
            .unwrap(),
        })
    }

    #[rstest]
    #[case(constant())]
    fn test_category(#[case] proc: Process<VariableTypesForData>) {
        let cat = proc.category();

        assert_eq!(
            cat,
            crate::products::general::core::ComponentCategory::Process
        );
    }

    #[rstest]
    #[case(constant())]
    #[case(deterministic())]
    #[case(market())]
    fn test_depends_on(#[case] proc: Process<VariableTypesForData>) {
        let expected: HashSet<_> = match &proc {
            Process::ConstantFloat(c) => c.depends_on().into_iter().collect(),
            Process::DeterministicFloat(d) => d.depends_on().into_iter().collect(),
            Process::MarketRef(m) => m.depends_on().into_iter().collect(),
        };

        let deps: HashSet<_> = proc.depends_on().into_iter().collect();

        assert_eq!(deps, expected);
    }
}
