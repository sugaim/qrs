use qmath::num::Positive;

use crate::quantity::CcyPair;

// -----------------------------------------------------------------------------
// FxRate
// -----------------------------------------------------------------------------
#[derive(
    Debug, Clone, Copy, PartialEq, serde::Serialize, serde::Deserialize, schemars::JsonSchema,
)]
#[serde(bound(deserialize = "Positive<V>: serde::Deserialize<'de>"))]
pub struct FxRate<V> {
    pub pair: CcyPair,
    pub value: Positive<V>,
}
