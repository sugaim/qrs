use qmath::num::Real;

// -----------------------------------------------------------------------------
// LnCoord
// StrikeDer
// -----------------------------------------------------------------------------
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct LnCoord<V>(pub V);

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
// LnVolSlice
// -----------------------------------------------------------------------------
pub trait LnVolSlice {
    type Value: Real;

    /// Calculate volatility at the given log money-ness
    fn lnvol(&self, coord: &LnCoord<Self::Value>) -> anyhow::Result<Self::Value>;

    /// Calculate 0th, 1st, and 2nd derivative of volatility at the given log money-ness
    fn lnvol_der(&self, coord: &LnCoord<Self::Value>) -> anyhow::Result<StrikeDer<Self::Value>>;
}
