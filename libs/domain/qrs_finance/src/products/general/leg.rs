mod straight;

use crate::products::general::core::VariableTypes;

use derivative::Derivative;
use qrs_finance_derive::Component;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
pub use straight::StraightLeg;

// -----------------------------------------------------------------------------
// Leg
//
#[derive(Derivative, Component, Serialize, Deserialize, JsonSchema)]
#[derivative(
    Debug(bound = "StraightLeg<Ts>: std::fmt::Debug"),
    Clone(bound = "StraightLeg<Ts>: Clone"),
    PartialEq(bound = "StraightLeg<Ts>: PartialEq")
)]
#[serde(
    tag = "type",
    rename_all = "snake_case",
    bound(
        serialize = "StraightLeg<Ts>: Serialize",
        deserialize = "StraightLeg<Ts>: Deserialize<'de>"
    )
)]
#[schemars(bound = "Ts: JsonSchema, StraightLeg<Ts>: JsonSchema")]
pub enum Leg<Ts: VariableTypes> {
    Straight(StraightLeg<Ts>),
}

// =============================================================================
#[cfg(test)]
mod tests {
    use std::collections::HashSet;

    use rstest::rstest;

    use crate::products::general::{
        core::{Component, ComponentCategory, ValueLess, WithId},
        VariableTypesForData,
    };

    use super::*;

    fn straight() -> Leg<VariableTypesForData> {
        Leg::Straight(StraightLeg {
            cashflows: vec![
                WithId {
                    id: "cf1".to_string(),
                    value: ValueLess,
                },
                WithId {
                    id: "cf2".to_string(),
                    value: ValueLess,
                },
            ],
        })
    }

    #[rstest]
    #[case(straight())]
    fn test_category(#[case] leg: Leg<VariableTypesForData>) {
        let cat = leg.category();

        assert_eq!(cat, ComponentCategory::Leg);
    }

    #[rstest]
    #[case(straight())]
    fn test_depends_on(#[case] leg: Leg<VariableTypesForData>) {
        let expected: HashSet<_> = match &leg {
            Leg::Straight(l) => l.depends_on().into_iter().collect(),
        };

        let deps = leg.depends_on().into_iter().collect::<HashSet<_>>();

        assert_eq!(deps, expected);
    }
}
