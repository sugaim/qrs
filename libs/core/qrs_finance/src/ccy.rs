// -----------------------------------------------------------------------------
// Ccy
//
#[derive(
    Debug, Clone, Copy, PartialEq, Eq, Hash, strum::Display, strum::EnumIter, strum::EnumString,
)]
pub enum Ccy {
    JPY,
    USD,
}
