use std::fmt::Display;

use qchrono::{
    calendar::{CalendarSrc, CalendarSym},
    timepoint::Date,
};

use super::{Act360, Act365f, Bd252, YearFrac};

// -----------------------------------------------------------------------------
// DayCount
// -----------------------------------------------------------------------------
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum DayCount {
    Act365f,
    Act360,
    Bd252(Bd252),
}

impl From<Act365f> for DayCount {
    #[inline]
    fn from(_: Act365f) -> Self {
        DayCount::Act365f
    }
}

impl From<Act360> for DayCount {
    #[inline]
    fn from(_: Act360) -> Self {
        DayCount::Act360
    }
}

impl From<Bd252> for DayCount {
    #[inline]
    fn from(src: Bd252) -> Self {
        DayCount::Bd252(src)
    }
}

impl Display for DayCount {
    #[inline]
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let sym = self.symbol();
        write!(f, "{}", sym)
    }
}

impl YearFrac for DayCount {
    type Error = anyhow::Error;

    #[inline]
    fn year_frac(&self, start: &Date, end: &Date) -> anyhow::Result<f64> {
        match self {
            DayCount::Act365f => Act365f.year_frac(start, end).map_err(Into::into),
            DayCount::Act360 => Act360.year_frac(start, end).map_err(Into::into),
            DayCount::Bd252(src) => src.year_frac(start, end).map_err(Into::into),
        }
    }
}

impl DayCount {
    #[inline]
    pub fn symbol(&self) -> DayCountSym {
        match self {
            DayCount::Act365f => DayCountSym::Act365f,
            DayCount::Act360 => DayCountSym::Act360,
            DayCount::Bd252(src) => DayCountSym::Bd252 {
                calendar: src.calendar_sym().clone(),
            },
        }
    }
}

// -----------------------------------------------------------------------------
// DayCountSym
// -----------------------------------------------------------------------------
#[derive(
    Debug, Clone, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize, schemars::JsonSchema,
)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum DayCountSym {
    Act365f,
    Act360,
    Bd252 { calendar: CalendarSym },
}

impl Display for DayCountSym {
    #[inline]
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            DayCountSym::Act365f => write!(f, "act365f"),
            DayCountSym::Act360 => write!(f, "act360"),
            DayCountSym::Bd252 { calendar } => write!(f, "bd252[{}]", calendar),
        }
    }
}

// -----------------------------------------------------------------------------
// DayCountSrc
// -----------------------------------------------------------------------------
pub trait DayCountSrc {
    fn get_daycount(&self, sym: &DayCountSym) -> anyhow::Result<DayCount>;
}

impl<S: CalendarSrc> DayCountSrc for S {
    fn get_daycount(&self, sym: &DayCountSym) -> anyhow::Result<DayCount> {
        match sym {
            DayCountSym::Act365f => Ok(Act365f.into()),
            DayCountSym::Act360 => Ok(Act360.into()),
            DayCountSym::Bd252 { calendar } => {
                let cal = self.get_calendar(calendar)?;
                Ok(Bd252::new(calendar.clone(), cal).into())
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use qchrono::{
        calendar::{Calendar, CalendarSrc, CalendarSymAtom},
        ext::chrono::NaiveDate,
    };

    use super::*;

    struct MockCalendarSrc;

    impl CalendarSrc for MockCalendarSrc {
        fn get_calendar_atom(&self, req: &CalendarSymAtom) -> anyhow::Result<Calendar> {
            match req.as_str() {
                "TKY" | "NYC" => Calendar::builder()
                    .with_extra_business_days(Default::default())
                    .with_extra_holidays(Default::default())
                    .with_valid_period(NaiveDate::MIN, NaiveDate::MAX)
                    .build(),
                _ => Err(anyhow::anyhow!("unknown calendar")),
            }
        }
    }

    #[test]
    fn test_get_act365f() {
        let src = MockCalendarSrc;

        let res = src.get_daycount(&DayCountSym::Act365f).unwrap();

        assert_eq!(&res, &Act365f.into());
    }

    #[test]
    fn test_get_bd252() {
        let src = MockCalendarSrc;

        let res = src
            .get_daycount(&DayCountSym::Bd252 {
                calendar: "TKY|NYC".parse().unwrap(),
            })
            .unwrap();

        assert_eq!(
            &res,
            &DayCount::Bd252(Bd252::new(
                "TKY|NYC".parse().unwrap(),
                Calendar::builder()
                    .with_extra_business_days(Default::default())
                    .with_extra_holidays(Default::default())
                    .with_valid_period(NaiveDate::MIN, NaiveDate::MAX)
                    .build()
                    .unwrap()
            )),
        );
    }

    #[test]
    fn test_get_bd252_ng() {
        let src = MockCalendarSrc;

        let res = src.get_daycount(&DayCountSym::Bd252 {
            calendar: "XXX".parse().unwrap(),
        });

        assert!(res.is_err());
    }
}
