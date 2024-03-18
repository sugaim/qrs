use qrs_finance_derive::Component;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

// -----------------------------------------------------------------------------
// OvernightRate
//
#[derive(Debug, Clone, PartialEq, Eq, Hash, Component, Serialize, Deserialize, JsonSchema)]
#[component(category = "Market")]
pub struct OvernightRate {
    pub reference: crate::market::ir::OvernightRate,
}

// =============================================================================
#[cfg(test)]
mod tests {
    use crate::products::general::core::{Component, ComponentCategory};

    use super::*;

    #[test]
    fn test_category() {
        let or = OvernightRate {
            reference: crate::market::ir::OvernightRate::TONA,
        };

        let cat = or.category();

        assert_eq!(cat, ComponentCategory::Market);
    }

    #[test]
    fn test_depends_on() {
        let or = OvernightRate {
            reference: crate::market::ir::OvernightRate::TONA,
        };

        let deps = or.depends_on();

        assert!(deps.into_iter().next().is_none());
    }
}
