use qrs_collections::NonEmpty;
use qrs_finance_derive::Component;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::products::general::core::VariableTypes;

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
pub struct ConstantFloat<Ts: VariableTypes> {
    #[component(field(category = "Constant"))]
    pub values: NonEmpty<Vec<Ts::Number>>,
}

// =============================================================================
#[cfg(test)]
mod tests {
    use qrs_collections::RequireMinSize;

    use crate::products::general::core::{Component, ComponentCategory, ValueOrId};
    use crate::products::general::VariableTypesForData;

    use super::*;

    fn proc() -> ConstantFloat<VariableTypesForData> {
        ConstantFloat {
            values: vec![
                ValueOrId::Id("cf1".to_string()),
                ValueOrId::Value(0.42),
                ValueOrId::Id("cf3".to_string()),
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
