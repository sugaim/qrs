use qrs_collections::NonEmpty;
use qrs_finance_derive::Component;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::product::general::core::{VariableTypes, WithId};

// -----------------------------------------------------------------------------
// MarketRef
//
#[derive(Debug, Clone, PartialEq, Eq, Hash, Component, Serialize, Deserialize, JsonSchema)]
#[component(category = "Process")]
#[serde(bound(
    serialize = "WithId<Ts::MarketRef>: Serialize",
    deserialize = "WithId<Ts::MarketRef>: Deserialize<'de>"
))]
#[schemars(bound = "Ts: JsonSchema, WithId<Ts::MarketRef>: JsonSchema")]
pub struct MarketRef<Ts: VariableTypes> {
    #[has_dependency(ref_category = "Market")]
    #[serde(rename = "references")]
    pub refs: NonEmpty<Vec<WithId<Ts::MarketRef>>>,
}

// =============================================================================
#[cfg(test)]
mod tests {
    use qrs_collections::RequireMinSize;

    use crate::product::general::{
        core::{Component, ComponentCategory, HasDependency, ValueLess},
        VariableTypesForData,
    };

    use super::*;

    fn mr() -> MarketRef<VariableTypesForData> {
        MarketRef {
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
        }
    }

    #[test]
    fn test_category() {
        let mr = mr();

        let cat = mr.category();

        assert_eq!(cat, ComponentCategory::Process);
    }

    #[test]
    fn test_depends_on() {
        let mr = mr();

        let deps = mr.depends_on().into_iter().collect::<Vec<_>>();

        assert_eq!(
            deps,
            vec![
                ("mr1", ComponentCategory::Market),
                ("mr2", ComponentCategory::Market)
            ]
        );
    }
}
