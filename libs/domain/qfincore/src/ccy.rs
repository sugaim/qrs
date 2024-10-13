use qmath::num::Positive;

// -----------------------------------------------------------------------------
// Ccy
// -----------------------------------------------------------------------------
#[derive(
    Debug,
    Clone,
    Copy,
    PartialEq,
    Eq,
    Hash,
    serde::Serialize,
    serde::Deserialize,
    schemars::JsonSchema,
    strum::Display,
)]
#[serde(rename_all = "UPPERCASE")]
pub enum Ccy {
    JPY,
    USD,
    EUR,
}

// -----------------------------------------------------------------------------
// CcyPair
// -----------------------------------------------------------------------------
#[derive(
    Debug,
    Clone,
    Copy,
    PartialEq,
    Eq,
    Hash,
    serde::Serialize,
    serde::Deserialize,
    schemars::JsonSchema,
)]
pub struct CcyPair {
    pub base: Ccy,
    pub quote: Ccy,
}

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
