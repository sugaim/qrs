use qrs_collections::NonEmpty;
use qrs_finance_derive::Component;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::product::general::core::VariableTypes;

// -----------------------------------------------------------------------------
// ConstantFloat
//
#[derive(Debug, Clone, PartialEq, Component, Serialize, Deserialize, JsonSchema)]
#[component(category = "Process")]
#[serde(bound(
    serialize = "Ts::Number: Serialize",
    deserialize = "Ts::Number: Deserialize<'de>"
))]
#[schemars(bound = "Ts: JsonSchema, Ts::Number: JsonSchema")]
pub struct ConstantNumber<Ts: VariableTypes> {
    #[has_dependency(ref_category = "Constant")]
    pub values: NonEmpty<Vec<Ts::Number>>,
}

// =============================================================================
#[cfg(test)]
mod tests {
    use qrs_collections::RequireMinSize;

    use crate::product::general::core::{Component, ComponentCategory, HasDependency, ValueOrId};
    use crate::product::general::VariableTypesForData;

    use super::*;

    fn proc() -> ConstantNumber<VariableTypesForData> {
        ConstantNumber {
            values: vec![
                ValueOrId::Id("cf1".into()),
                ValueOrId::Value(0.42),
                ValueOrId::Id("cf3".into()),
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

        let deps = proc.depends_on().into_iter().collect::<Vec<_>>();

        assert_eq!(
            deps,
            vec![
                ("cf1", ComponentCategory::Constant),
                ("cf3", ComponentCategory::Constant)
            ]
        );
    }
}
