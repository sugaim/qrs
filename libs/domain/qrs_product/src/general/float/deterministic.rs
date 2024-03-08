use std::collections::HashMap;

use qrs_chrono::DateWithTag;
use qrs_collections::NonEmpty;

// -----------------------------------------------------------------------------
// Deterministic
//
#[derive(Debug, Clone, PartialEq)]
#[cfg_attr(
    feature = "serde",
    derive(serde::Serialize, serde::Deserialize, schemars::JsonSchema)
)]
pub struct Deterministic {
    pub series: NonEmpty<Vec<NonEmpty<HashMap<DateWithTag, f64>>>>,
}
