use std::collections::HashMap;

use derivative::Derivative;
use qrs_collections::NonEmpty;
use qrs_finance_derive::Component;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::product::general::core::VariableTypes;

// -----------------------------------------------------------------------------
// Deterministic
//
#[derive(Debug, Clone, Derivative, Component, Serialize, Deserialize, JsonSchema)]
#[component(category = "Process")]
#[derivative(PartialEq(bound = "Ts::DateTime: Eq + std::hash::Hash, Ts::Number: PartialEq  "))]
#[serde(bound(
    serialize = "Ts::DateTime: Eq + std::hash::Hash + Serialize, Ts::Number: Serialize",
    deserialize = "Ts::DateTime: Eq + std::hash::Hash + Deserialize<'de>, Ts::Number: Deserialize<'de>"
))]
#[schemars(bound = "Ts: JsonSchema, Ts::Number: JsonSchema, Ts::DateTime: JsonSchema")]
pub struct DeterministicFloat<Ts: VariableTypes> {
    #[has_dependency(ref_category = "Constant")]
    #[allow(clippy::type_complexity)]
    pub series: NonEmpty<Vec<NonEmpty<HashMap<Ts::DateTime, Ts::Number>>>>,
}

// =============================================================================
#[cfg(test)]
mod tests {
    use std::collections::HashSet;

    use maplit::{hashmap, hashset};
    use qrs_collections::RequireMinSize;

    use crate::product::general::core::{Component, ComponentCategory, HasDependency, ValueOrId};
    use crate::product::general::VariableTypesForData;

    use super::*;

    fn proc() -> DeterministicFloat<VariableTypesForData> {
        DeterministicFloat {
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
        }
    }

    #[test]
    fn test_category() {
        let proc = proc();

        let cat = proc.category();

        assert_eq!(cat, ComponentCategory::Process);
    }

    #[test]
    fn test_depends_on() {
        let proc = proc();

        let deps = proc.depends_on().into_iter().collect::<HashSet<_>>();

        assert_eq!(
            deps,
            hashset! {
                ("datetime1", ComponentCategory::Constant),
                ("datetime2", ComponentCategory::Constant),
                ("num1", ComponentCategory::Constant),
                ("num2", ComponentCategory::Constant)
            }
        );
    }
}
