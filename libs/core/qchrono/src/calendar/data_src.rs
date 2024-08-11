use std::collections::HashMap;

use smallvec::SmallVec;

use super::{Calendar, CalendarSym, CalendarSymAtom};

// -----------------------------------------------------------------------------
// CalendarSrc
// CalendarSrcInduce
// -----------------------------------------------------------------------------
pub trait CalendarSrc {
    fn get_calendar(&self, req: &CalendarSym) -> anyhow::Result<Calendar>;
}

pub trait CalendarSrcInduce {
    fn get_calendar_atom(&self, req: &CalendarSymAtom) -> anyhow::Result<Calendar>;
}

impl<S: CalendarSrcInduce> CalendarSrc for S {
    fn get_calendar(&self, req: &CalendarSym) -> anyhow::Result<Calendar> {
        let leaves = req
            .leaves()
            .into_iter()
            .map(|s| self.get_calendar_atom(&s).map(|cal| (s, cal)))
            .collect::<anyhow::Result<HashMap<_, _>>>()?;
        let data = _merge_leaves(req, &leaves)?;
        Ok(data)
    }
}

fn _merge_leaves(
    sym: &CalendarSym,
    leaves: &HashMap<CalendarSymAtom, Calendar>,
) -> anyhow::Result<Calendar> {
    match sym {
        CalendarSym::Single(name) => leaves.get(name).cloned().ok_or_else(|| {
            anyhow::anyhow!("unexpected: '{name}' is not found in prefetched calendars.")
        }),
        CalendarSym::AllClosed(syms) | CalendarSym::AnyClosed(syms) => {
            let cals = syms
                .iter()
                .map(|s| _merge_leaves(s, leaves))
                .collect::<Result<SmallVec<[Calendar; 5]>, _>>()?;
            let res = match sym {
                CalendarSym::AllClosed(_) => Calendar::all_closed_of(cals),
                CalendarSym::AnyClosed(_) => Calendar::any_closed_of(cals),
                _ => unreachable!(),
            };
            Ok(res.expect("A collection 'syms' is not empty. So merged calendar must be created."))
        }
    }
}

#[cfg(test)]
mod tests {
    use chrono::NaiveDate;
    use maplit::hashmap;
    use rstest::rstest;

    use super::*;

    mockall::mock! {
        Src {}

        impl CalendarSrcInduce for Src {
            fn get_calendar_atom(&self, req: &CalendarSymAtom) -> anyhow::Result<Calendar>;
        }
    }

    #[derive(Default)]
    struct CallCount {
        get: Option<usize>,
    }

    fn get_cal(nm: &CalendarSymAtom) -> anyhow::Result<Calendar> {
        let data = hashmap! {
            CalendarSymAtom::new("TKY").unwrap() => Calendar::builder()
                .with_valid_period(
                    NaiveDate::from_ymd_opt(2000, 1, 1).unwrap(),
                    NaiveDate::from_ymd_opt(2999, 12, 31).unwrap(),
                )
                .with_extra_holidays(vec![NaiveDate::from_ymd_opt(2021, 1, 1).unwrap()])
                .with_extra_business_days(vec![])
                .build()
                .unwrap(),
            CalendarSymAtom::new("NYK").unwrap() => Calendar::builder()
                .with_valid_period(
                    NaiveDate::from_ymd_opt(2000, 1, 1).unwrap(),
                    NaiveDate::from_ymd_opt(2500, 12, 31).unwrap(),
                )
                .with_extra_holidays(vec![NaiveDate::from_ymd_opt(2021, 1, 13).unwrap()])
                .with_extra_business_days(vec![])
                .build()
                .unwrap(),
            CalendarSymAtom::new("LDN").unwrap() => Calendar::builder()
                .with_valid_period(
                    NaiveDate::from_ymd_opt(2000, 1, 1).unwrap(),
                    NaiveDate::from_ymd_opt(2200, 12, 31).unwrap(),
                )
                .with_extra_holidays(vec![NaiveDate::from_ymd_opt(2021, 1, 6).unwrap()])
                .with_extra_business_days(vec![])
                .build()
                .unwrap(),
        };
        data.get(nm)
            .cloned()
            .ok_or_else(|| anyhow::anyhow!("not found"))
    }

    impl MockSrc {
        fn with_call_count(call_count: &CallCount) -> Self {
            let mut mock = MockSrc::new();
            mock.setup(call_count);
            mock
        }

        fn setup(&mut self, call_count: &CallCount) {
            //
            let get = self.expect_get_calendar_atom().returning(get_cal);
            if let Some(count) = call_count.get {
                get.times(count);
            }
        }
    }

    #[rstest]
    #[case(
        "NYK".parse().unwrap(), 1,
        Ok(get_cal(&"NYK".parse().unwrap()).unwrap())
    )]
    #[case(
        "NYK|TKY".parse().unwrap(), 2,
        Ok(get_cal(&"NYK".parse().unwrap()).unwrap() | get_cal(&"TKY".parse().unwrap()).unwrap())
    )]
    #[case(
        "NYK&LDN".parse().unwrap(), 2,
        Ok(get_cal(&"NYK".parse().unwrap()).unwrap() & get_cal(&"LDN".parse().unwrap()).unwrap())
    )]
    #[case(
        "NYK|TKY&LDN".parse().unwrap(), 3,
        Ok(get_cal(&"NYK".parse().unwrap()).unwrap() | (get_cal(&"TKY".parse().unwrap()).unwrap() & get_cal(&"LDN".parse().unwrap()).unwrap()))
    )]
    #[case(
        "XXX".parse().unwrap(), 1,
        Err("not found".to_owned())
    )]
    #[case(
        "XXX|NYK".parse().unwrap(), 2,
        Err("not found".to_owned())
    )]
    fn test_get(
        #[case] sym: CalendarSym,
        #[case] num: usize,
        #[case] exp: Result<Calendar, String>,
    ) {
        let mut mock = MockSrc::with_call_count(&CallCount { get: Some(num) });

        let res = mock.get_calendar(&sym);

        assert_eq!(res.map_err(|e| e.to_string()), exp);
        mock.checkpoint();
    }
}
