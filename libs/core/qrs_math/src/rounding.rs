use rust_decimal::Decimal;

// -----------------------------------------------------------------------------
// RoundingStrategy
//
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[cfg_attr(
    feature = "serde",
    derive(serde::Serialize, serde::Deserialize, schemars::JsonSchema),
    serde(rename_all = "snake_case")
)]
pub enum RoundingStrategy {
    NearestEven,
    NearestInteger,
    ToZero,
    AwayFromZero,
    ToNegativeInfinity,
    ToPositiveInfinity,
}

// -----------------------------------------------------------------------------
// Rounding
//
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[cfg_attr(
    feature = "serde",
    derive(serde::Serialize, serde::Deserialize, schemars::JsonSchema)
)]
pub struct Rounding {
    pub scale: u32,
    pub strategy: RoundingStrategy,
}

//
// methods
//
impl Rounding {
    #[inline]
    pub fn apply(&self, value: &Decimal) -> Decimal {
        use RoundingStrategy::*;
        let stragegy = match self.strategy {
            NearestEven => rust_decimal::RoundingStrategy::MidpointNearestEven,
            NearestInteger => rust_decimal::RoundingStrategy::MidpointAwayFromZero,
            ToZero => rust_decimal::RoundingStrategy::ToZero,
            AwayFromZero => rust_decimal::RoundingStrategy::AwayFromZero,
            ToNegativeInfinity => rust_decimal::RoundingStrategy::ToNegativeInfinity,
            ToPositiveInfinity => rust_decimal::RoundingStrategy::ToPositiveInfinity,
        };
        value.round_dp_with_strategy(self.scale, stragegy)
    }
}
