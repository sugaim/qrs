// -----------------------------------------------------------------------------
// LnCoord
// IvStrikeDer
// -----------------------------------------------------------------------------
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct LnCoord<V>(pub V);

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct VarianceStrikeDer<V> {
    /// Value, i.e., implied variance
    pub var: V,

    /// First derivative of implied volatility w.r.t. log money-ness
    pub dvdy: V,

    /// Second derivative of implied volatility w.r.t. log money-ness
    pub d2vdy2: V,
}

// -----------------------------------------------------------------------------
// LnVolSlice
// -----------------------------------------------------------------------------
pub trait LnVolSlice {
    type Value;

    /// Calculate implied volatility at the given log money-ness
    fn iv(&self, coord: &LnCoord<Self::Value>) -> anyhow::Result<Self::Value>;

    /// Calculate 0th, 1st, and 2nd derivative of implied volatility at the given log money-ness
    fn variance_der(
        &self,
        coord: &LnCoord<Self::Value>,
    ) -> anyhow::Result<VarianceStrikeDer<Self::Value>>;
}
