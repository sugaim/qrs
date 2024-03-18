use qrs_finance_derive::Component;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::product::general::core::{VariableTypes, WithId};

// -----------------------------------------------------------------------------
// StraightLeg
//
#[derive(Debug, Clone, PartialEq, Component, Serialize, Deserialize, JsonSchema)]
#[component(category = "Leg")]
#[serde(
    tag = "type",
    rename_all = "snake_case",
    bound(
        serialize = "WithId<Ts::CashflowRef>: Serialize",
        deserialize = "WithId<Ts::CashflowRef>: Deserialize<'de>"
    )
)]
#[schemars(bound = "Ts: JsonSchema, WithId<Ts::CashflowRef>: JsonSchema")]
pub struct StraightLeg<Ts: VariableTypes> {
    #[has_dependency(ref_category = "Cashflow")]
    pub cashflows: Vec<WithId<Ts::CashflowRef>>,
}

// =============================================================================
#[cfg(test)]
mod tests {
    use crate::product::general::{
        core::{Component, ComponentCategory, HasDependency, ValueLess},
        VariableTypesForData,
    };

    use super::*;

    fn leg() -> StraightLeg<VariableTypesForData> {
        StraightLeg {
            cashflows: vec![
                WithId {
                    id: "cf1".into(),
                    value: ValueLess,
                },
                WithId {
                    id: "cf2".into(),
                    value: ValueLess,
                },
            ],
        }
    }

    #[test]
    fn test_category() {
        let leg = leg();

        let cat = leg.category();

        assert_eq!(cat, ComponentCategory::Leg);
    }

    #[test]
    fn test_depends_on() {
        let leg = leg();

        let deps = leg.depends_on();

        let deps: Vec<_> = deps.into_iter().collect();

        assert_eq!(
            deps,
            vec![
                ("cf1", ComponentCategory::Cashflow),
                ("cf2", ComponentCategory::Cashflow)
            ]
        );
    }
}
