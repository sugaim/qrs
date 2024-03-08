use qrs_collections::NonEmpty;

// -----------------------------------------------------------------------------
// Constant
//
#[derive(Debug, Clone)]
#[cfg_attr(
    feature = "serde",
    derive(serde::Serialize, serde::Deserialize, schemars::JsonSchema)
)]
pub struct Constant {
    pub values: NonEmpty<Vec<f64>>,
}
