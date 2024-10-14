use qchrono::timepoint::DateTime;
use qfincore::quantity::{CcyPair, FxRate};
use qmath::num::Real;

// -----------------------------------------------------------------------------
// FxSpot
// -----------------------------------------------------------------------------
#[derive(Debug, Clone, PartialEq, serde::Serialize, schemars::JsonSchema)]
pub struct FxSpot<V> {
    pub spot_date: DateTime,
    pub rate: FxRate<V>,
}

// -----------------------------------------------------------------------------
// FxSpotSrc
// -----------------------------------------------------------------------------
pub trait FxSpotSrc {
    type Value: Real;

    fn get_fxspot(&self, pair: &CcyPair) -> anyhow::Result<FxSpot<Self::Value>>;
}
