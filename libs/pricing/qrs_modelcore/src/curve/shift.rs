use qrs_chrono::{DateTime, Duration};
use qrs_finance::rate::RateAct365f;
#[cfg(feature = "serde")]
use schemars::JsonSchema;
#[cfg(feature = "serde")]
use serde::{Deserialize, Serialize};

use super::{YieldCurve, YieldCurveAdjust};

// -----------------------------------------------------------------------------
// Shift
//

/// Shift curve w.r.t. time direction.
///
/// The shift is performed with moving the curve shape to forward direction.
/// ```txt
///       o--------------
///       |
/// ------x
/// ------t--------------> time
///
///         |
///         | shift with +2d
///         V
///              o-------
///              |
/// -------------x
/// ------t-----t+2d-----> time
/// ```
///
/// As mathematical expression, shifted curve returns
/// `yc.forward_rate(from - dt, to - dt)` where `dt` is the shift duration
/// and `yc` is the original curve.
///
#[derive(Debug, Clone, PartialEq)]
#[cfg_attr(
    feature = "serde",
    derive(Serialize, Deserialize, JsonSchema),
    schemars(description = "Shift curve w.r.t. time direction.")
)]
pub struct Shift {
    /// Shift size.
    pub dt: Duration,
}

//
// methods
//
impl<C: YieldCurve> YieldCurveAdjust<C> for Shift {
    #[inline]
    fn adjusted_forward_rate(
        &self,
        curve: &C,
        from: &DateTime,
        to: &DateTime,
    ) -> anyhow::Result<RateAct365f<<C as YieldCurve>::Value>> {
        let from = *from - self.dt;
        let to = *to - self.dt;
        curve.forward_rate(&from, &to)
    }
}
