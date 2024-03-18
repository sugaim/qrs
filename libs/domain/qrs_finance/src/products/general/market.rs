mod overnight_rate;

use qrs_finance_derive::Component;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

pub use overnight_rate::OvernightRate;

// -----------------------------------------------------------------------------
// Market
//
#[derive(Debug, Clone, PartialEq, Eq, Hash, Component, Serialize, Deserialize, JsonSchema)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum Market {
    OvernightRate(OvernightRate),
}

// =============================================================================
#[cfg(test)]
mod tests {
    use std::collections::HashSet;

    use rstest::rstest;

    use crate::products::general::core::Component;

    use super::*;

    fn on_rate() -> Market {
        Market::OvernightRate(OvernightRate {
            reference: crate::market::ir::OvernightRate::TONA,
        })
    }

    #[rstest]
    #[case(on_rate())]
    fn test_category(#[case] mkt: Market) {
        let cat = mkt.category();

        assert_eq!(
            cat,
            crate::products::general::core::ComponentCategory::Market
        );
    }

    #[rstest]
    #[case(on_rate())]
    fn test_depends_on(#[case] mkt: Market) {
        let expected: HashSet<_> = match &mkt {
            Market::OvernightRate(or) => or.depends_on().into_iter().collect(),
        };

        let deps: HashSet<_> = mkt.depends_on().into_iter().collect();

        assert_eq!(deps, expected);
    }
}
