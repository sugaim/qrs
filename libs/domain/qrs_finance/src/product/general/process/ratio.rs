use derivative::Derivative;
use qrs_finance_derive::Component;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::product::general::core::{VariableTypes, WithId};

// -----------------------------------------------------------------------------
// Ratio
//
#[derive(Derivative, Component, Serialize, Deserialize, JsonSchema)]
#[component(category = "Process")]
#[derivative(
    Debug(bound = "WithId<Ts::ProcessRef>: std::fmt::Debug"),
    Clone(bound = "WithId<Ts::ProcessRef>: Clone"),
    PartialEq(bound = "WithId<Ts::ProcessRef>: PartialEq")
)]
#[serde(bound(
    serialize = "WithId<Ts::ProcessRef>: Serialize",
    deserialize = "WithId<Ts::ProcessRef>: Deserialize<'de>"
))]
#[schemars(bound = "Ts: JsonSchema, WithId<Ts::ProcessRef>: JsonSchema")]
pub struct Ratio<Ts: VariableTypes> {
    #[has_dependency(ref_category = "Process")]
    pub numer: WithId<Ts::ProcessRef>,

    #[has_dependency(ref_category = "Process")]
    pub denom: WithId<Ts::ProcessRef>,
}

// =============================================================================
#[cfg(test)]
mod tests {
    use std::collections::HashSet;

    use maplit::hashset;

    use crate::product::general::{
        core::{Component, ComponentCategory, HasDependency, ValueLess},
        VariableTypesForData,
    };

    use super::*;

    fn ratio() -> Ratio<VariableTypesForData> {
        Ratio {
            numer: WithId {
                id: "numer".into(),
                value: ValueLess,
            },
            denom: WithId {
                id: "denom".into(),
                value: ValueLess,
            },
        }
    }

    #[test]
    fn test_category() {
        let ratio = ratio();

        let cat = ratio.category();

        assert_eq!(cat, ComponentCategory::Process);
    }

    #[test]
    fn test_depends_on() {
        let ratio = ratio();

        let deps = ratio.depends_on().into_iter().collect::<HashSet<_>>();

        assert_eq!(
            deps,
            hashset! {
                ("numer", ComponentCategory::Process),
                ("denom", ComponentCategory::Process)
            }
        );
    }
}
