use qchrono::timepoint::DateTime;
use qfincore::{CcyPair, FxRate};
use qmath::num::Real;

use super::super::spot::FxSpot;

// -----------------------------------------------------------------------------
// FxFwd
// -----------------------------------------------------------------------------
pub trait FxFwd {
    type Value: Real;

    /// Return the spot rate which this forward assumes.
    fn fxspot(&self) -> FxSpot<Self::Value>;

    /// Calculate the forward exchange rate at the given target date.
    ///
    /// Be careful that this is distinguished from the forward spot rate which is
    /// calculated with [FxFwd::fwdspot_of] method.
    /// This method calculates the forward exchange rate with the given target date.
    /// So returns spot rate if the target date is the spot date.
    fn forward_of(
        &self,
        spot_rate: &Self::Value,
        spot_date: &DateTime,
        tgt: &DateTime,
    ) -> anyhow::Result<FxRate<Self::Value>>;

    /// Calculate the forward exchange rate at the given target date
    /// with the spot rate held by this forward.
    ///
    /// Please see [FxFwd::fwdpx_of] for more details.
    #[inline]
    fn forward(&self, tgt: &DateTime) -> anyhow::Result<FxRate<Self::Value>> {
        self.forward_of(&self.fxspot().rate.value, &self.fxspot().spot_date, tgt)
    }

    /// Calculate the forward spot rate at the given target date.
    ///
    /// This method calculates the forward spot rate with the given target date
    /// rather than the forward exchange rate.
    /// So returns spot rate if the target date is today (and the today's
    /// spot date matches with the one of the argument spot).
    fn fwdspot_of(
        &self,
        spot_rate: &Self::Value,
        spot_date: &DateTime,
        tgt: &DateTime,
    ) -> anyhow::Result<FxRate<Self::Value>>;

    /// Calculate the forward spot rate at the given target date
    ///
    /// Please see [FxFwd::fwdspot_of] for more details.
    #[inline]
    fn fwdspot(&self, tgt: &DateTime) -> anyhow::Result<FxRate<Self::Value>> {
        self.fwdspot_of(&self.fxspot().rate.value, &self.fxspot().spot_date, tgt)
    }
}

// -----------------------------------------------------------------------------
// FxFwdSrc
// -----------------------------------------------------------------------------
pub trait FxFwdSrc {
    type FxFwd: FxFwd;

    fn get_fxfwd(&self, req: &CcyPair) -> anyhow::Result<Self::FxFwd>;
}
