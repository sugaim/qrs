use qmath::num::Real;

use crate::lnvol::LnCoord;

// -----------------------------------------------------------------------------
// LnCoord
// StrikeDer
// -----------------------------------------------------------------------------

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct StrikeDer<V> {
    /// 0-th derivative of volatility w.r.t. log money-ness
    pub vol: V,

    /// First derivative of volatility w.r.t. log money-ness
    pub dvdy: V,

    /// Second derivative of volatility w.r.t. log money-ness
    pub d2vdy2: V,
}

// -----------------------------------------------------------------------------
// VolCurve
// -----------------------------------------------------------------------------
pub trait VolCurve {
    type Value: Real;

    /// Calculate total volatility at the given log money-ness
    ///
    /// Here, the total volatility is dimentionless value, sigma * sqrt(T),
    /// where sigma is the volatility and T is the time to maturity.
    /// Implementation must ensure that this value is non-negative.
    fn bs_totalvol(&self, coord: &LnCoord<Self::Value>) -> anyhow::Result<Self::Value>;

    /// Calculate 0th, 1st, and 2nd derivative of volatility at the given log money-ness
    fn bsvol_der(&self, coord: &LnCoord<Self::Value>) -> anyhow::Result<StrikeDer<Self::Value>>;
}
