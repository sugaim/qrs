mod _ops;
mod act360;
mod act365f;
mod bd252;
mod nl360;
mod nl365;
mod traits;

use qrs_chrono::{Calendar, CalendarSymbol, NaiveDate};
use qrs_datasrc::{DataSrc, DebugTree};
use qrs_math::num::Real;

pub use act360::{Act360, Act360Rate};
pub use act365f::{Act365f, Act365fRate};
pub use bd252::{Bd252, Bd252Rate};
pub use nl360::{Nl360, Nl360Rate};
pub use nl365::{Nl365, Nl365Rate};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
pub use traits::{Dcf, DcfError, InterestRate, RateDcf};

// -----------------------------------------------------------------------------
// Rate
//
#[derive(Debug, Clone, PartialEq)]
pub enum Rate<V> {
    Act360(Act360Rate<V>),
    Act365f(Act365fRate<V>),
    Nl360(Nl360Rate<V>),
    Nl365(Nl365Rate<V>),
    Bd252(Bd252Rate<V>),
}

//
// methods
//
impl<V: Real> InterestRate for Rate<V> {
    type Value = V;
    type Convention = DayCount;

    #[inline]
    fn convention(&self) -> Self::Convention {
        match self {
            Rate::Act360(rate) => DayCount::Act360(rate.convention()),
            Rate::Act365f(rate) => DayCount::Act365f(rate.convention()),
            Rate::Nl360(rate) => DayCount::Nl360(rate.convention()),
            Rate::Nl365(rate) => DayCount::Nl365(rate.convention()),
            Rate::Bd252(rate) => DayCount::Bd252(rate.convention()),
        }
    }

    #[inline]
    fn into_value(self) -> Self::Value {
        match self {
            Rate::Act360(rate) => rate.into_value(),
            Rate::Act365f(rate) => rate.into_value(),
            Rate::Nl360(rate) => rate.into_value(),
            Rate::Nl365(rate) => rate.into_value(),
            Rate::Bd252(rate) => rate.into_value(),
        }
    }
}

// -----------------------------------------------------------------------------
// DayCount
//
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DayCount {
    Act360(Act360),
    Act365f(Act365f),
    Nl360(Nl360),
    Nl365(Nl365),
    Bd252(Bd252),
}

//
// methods
//
impl Dcf for DayCount {
    #[inline]
    fn dcf(&self, from: NaiveDate, to: NaiveDate) -> Result<f64, DcfError> {
        match self {
            DayCount::Act360(dcf) => dcf.dcf(from, to),
            DayCount::Act365f(dcf) => dcf.dcf(from, to),
            DayCount::Nl360(dcf) => dcf.dcf(from, to),
            DayCount::Nl365(dcf) => dcf.dcf(from, to),
            DayCount::Bd252(dcf) => dcf.dcf(from, to),
        }
    }
}

impl RateDcf for DayCount {
    type Rate<V: Real> = Rate<V>;

    #[inline]
    fn to_rate<V: Real>(&self, annual_rate: V) -> Self::Rate<V> {
        match self {
            DayCount::Act360(dcf) => Rate::Act360(dcf.to_rate(annual_rate)),
            DayCount::Act365f(dcf) => Rate::Act365f(dcf.to_rate(annual_rate)),
            DayCount::Nl360(dcf) => Rate::Nl360(dcf.to_rate(annual_rate)),
            DayCount::Nl365(dcf) => Rate::Nl365(dcf.to_rate(annual_rate)),
            DayCount::Bd252(dcf) => Rate::Bd252(dcf.to_rate(annual_rate)),
        }
    }
}

// -----------------------------------------------------------------------------
// DayCountSymbol
//
#[derive(Debug, Clone, PartialEq, Eq, strum::Display, Serialize, Deserialize, JsonSchema)]
#[serde(tag = "type")]
pub enum DayCountSymbol {
    #[serde(rename = "act360")]
    #[strum(serialize = "act360")]
    Act360,
    #[serde(rename = "act365f")]
    #[strum(serialize = "act365f")]
    Act365f,
    #[serde(rename = "nl360")]
    #[strum(serialize = "nl360")]
    Nl360,
    #[serde(rename = "nl365")]
    #[strum(serialize = "nl365")]
    Nl365,
    #[serde(rename = "bd252")]
    #[strum(serialize = "bd252")]
    Bd252 {
        #[serde(rename = "calendar")]
        cal: CalendarSymbol,
    },
}

// -----------------------------------------------------------------------------
// DayCountSrc
//
#[derive(Debug, Clone, PartialEq, Eq, DebugTree)]
#[debug_tree(desc = "day count source")]
pub struct DayCountSrc<Cal> {
    #[debug_tree(subtree)]
    cal: Cal,
}

//
// construction
//
impl<Cal> DayCountSrc<Cal> {
    #[inline]
    pub fn new(cal: Cal) -> Self {
        Self { cal }
    }
}

//
// methods
//
impl<Cal> DataSrc<DayCountSymbol> for DayCountSrc<Cal>
where
    Cal: DataSrc<CalendarSymbol, Output = Calendar>,
{
    type Output = DayCount;

    #[inline]
    fn get(&self, req: &DayCountSymbol) -> anyhow::Result<Self::Output> {
        match req {
            DayCountSymbol::Act360 => Ok(DayCount::Act360(Act360)),
            DayCountSymbol::Act365f => Ok(DayCount::Act365f(Act365f)),
            DayCountSymbol::Nl360 => Ok(DayCount::Nl360(Nl360)),
            DayCountSymbol::Nl365 => Ok(DayCount::Nl365(Nl365)),
            DayCountSymbol::Bd252 { cal } => Ok(DayCount::Bd252(Bd252 {
                cal: self.cal.get(cal)?,
            })),
        }
    }
}
