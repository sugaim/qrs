// -----------------------------------------------------------------------------
// SingleCalendarSrc
//

use std::{
    borrow::Cow,
    collections::{hash_map::DefaultHasher, HashSet},
    hash::{Hash, Hasher},
};

use qrs_datasrc::{CacheableSrc, DataSrc, DebugTree, TakeSnapshot};

use super::CalendarSymVariant;

use super::{Calendar, CalendarSymbol};

// -----------------------------------------------------------------------------
// CalendarSrc
//

/// Data source for calendars
///
/// This data source split [`CalendarSymbol`] into single calendars
/// and combine corresponding calendars which are
/// retrieved from the inner data source.
///
/// Note that this data source does not use cache inside it,
/// although it may seem natural for someones because
/// recursive get is necessary due to the recursive structure of the symbol.
///
/// So if the user want to use cache, please wrap this data source with [`qrs_datasrc::CacheProxy`].
#[derive(Debug, DebugTree, Clone)]
#[debug_tree(desc = "calendar source")]
pub struct CalendarSrc<S> {
    #[debug_tree(subtree)]
    src: S,
}

//
// construction
//
impl<S: DataSrc<str, Output = Calendar>> CalendarSrc<S> {
    pub fn new(src: S) -> Self {
        Self { src }
    }
}

//
// methods
//
impl<S> CalendarSrc<S> {
    /// Get the inner data source.
    #[inline]
    pub fn inner(&self) -> &S {
        &self.src
    }

    /// Get the mutable reference to the inner data source.
    #[inline]
    pub fn inner_mut(&mut self) -> &mut S {
        &mut self.src
    }

    /// Unwrap the inner data source.
    #[inline]
    pub fn into_inner(self) -> S {
        self.src
    }
}

impl<S> DataSrc<CalendarSymbol> for CalendarSrc<S>
where
    S: DataSrc<str, Output = Calendar>,
{
    type Output = Calendar;

    fn get(&self, req: &CalendarSymbol) -> anyhow::Result<Self::Output> {
        use CalendarSymVariant::*;
        match req.dispatch() {
            Single(s) => self.src.get(s),
            AllClosed(syms) | AnyClosed(syms) => {
                let cals = syms.iter().map(|sym| self.get(sym).map(Cow::Owned));
                match req.dispatch() {
                    AllClosed(_) => Calendar::all_closed_try_from(cals),
                    _ => Calendar::any_closed_try_from(cals),
                }
            }
        }
    }
}

impl<S> CacheableSrc<CalendarSymbol> for CalendarSrc<S>
where
    S: CacheableSrc<str, Output = Calendar>,
{
    fn etag(&self, req: &CalendarSymbol) -> anyhow::Result<String> {
        use CalendarSymVariant::*;
        match req.dispatch() {
            Single(s) => {
                let mut hasher = DefaultHasher::new();
                s.hash(&mut hasher);
                self.src.etag(s)?.hash(&mut hasher);
                Ok(hasher.finish().to_string())
            }
            AllClosed(syms) | AnyClosed(syms) => {
                let mut hasher = DefaultHasher::new();
                matches!(req.dispatch(), &AnyClosed(_)).hash(&mut hasher);
                for sym in syms {
                    self.etag(sym)?.hash(&mut hasher);
                }
                Ok(hasher.finish().to_string())
            }
        }
    }
}

impl<S: TakeSnapshot<str, Output = Calendar>> TakeSnapshot<CalendarSymbol> for CalendarSrc<S> {
    type Snapshot = CalendarSrc<S::Snapshot>;

    fn take_snapshot<'a, Rqs>(&self, rqs: Rqs) -> anyhow::Result<Self::Snapshot>
    where
        CalendarSymbol: 'a,
        Rqs: IntoIterator<Item = &'a CalendarSymbol>,
    {
        let mut leaves = HashSet::new();
        for rq in rqs {
            rq.collect_leaves(&mut leaves);
        }
        let snapshot = self.src.take_snapshot(leaves.iter().map(|s| s.as_str()))?;
        Ok(CalendarSrc::new(snapshot))
    }
}

// =============================================================================
#[cfg(test)]
mod tests {
    use chrono::NaiveDate;
    use maplit::hashmap;
    use qrs_datasrc::{DataSrc, DebugTree, TreeInfo};
    use rstest::rstest;

    use super::*;

    mockall::mock! {
        Src {}

        impl DebugTree for Src {
            fn desc(&self) -> String;
            fn debug_tree(&self) -> TreeInfo;
        }

        impl DataSrc<str> for Src {
            type Output = Calendar;
            fn get(&self, req: &str) -> anyhow::Result<Calendar>;
        }

        impl CacheableSrc<str> for Src {
            fn etag(&self, req: &str) -> anyhow::Result<String>;
        }
    }

    #[derive(Default)]
    struct CallCount {
        desc: Option<usize>,
        debug_tree: Option<usize>,
        get: Option<usize>,
        etag: Option<usize>,
    }

    impl CallCount {
        fn zero() -> Self {
            CallCount {
                desc: Some(0),
                debug_tree: Some(0),
                get: Some(0),
                etag: Some(0),
            }
        }
    }

    fn get_cal(nm: &str) -> anyhow::Result<Calendar> {
        let data = hashmap! {
            "TKY" => Calendar::builder()
                .with_valid_period(
                    NaiveDate::from_ymd_opt(2000, 1, 1).unwrap(),
                    NaiveDate::from_ymd_opt(2999, 12, 31).unwrap(),
                )
                .with_extra_holidays(vec![NaiveDate::from_ymd_opt(2021, 1, 1).unwrap()])
                .with_extra_business_days(vec![])
                .build()
                .unwrap(),
            "NYK" => Calendar::builder()
                .with_valid_period(
                    NaiveDate::from_ymd_opt(2000, 1, 1).unwrap(),
                    NaiveDate::from_ymd_opt(2500, 12, 31).unwrap(),
                )
                .with_extra_holidays(vec![NaiveDate::from_ymd_opt(2021, 1, 13).unwrap()])
                .with_extra_business_days(vec![])
                .build()
                .unwrap(),
            "LDN" => Calendar::builder()
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
            let desc = self.expect_desc().return_const("mock".to_owned());
            if let Some(count) = call_count.desc {
                desc.times(count);
            }

            //
            let debug_tree = self.expect_debug_tree().return_const(TreeInfo::Leaf {
                desc: "mock".to_owned(),
                tp: std::any::type_name::<MockSrc>().to_owned(),
            });
            if let Some(count) = call_count.debug_tree {
                debug_tree.times(count);
            }

            //
            let get = self.expect_get().returning(get_cal);
            if let Some(count) = call_count.get {
                get.times(count);
            }

            //
            let etag = self.expect_etag().returning(|s| {
                let data = hashmap! {
                    "TKY" => "TKY".to_owned(),
                    "NYK" => "NYK".to_owned(),
                    "LDN" => "LDN".to_owned(),
                };
                data.get(s)
                    .cloned()
                    .ok_or_else(|| anyhow::anyhow!("not found"))
            });
            if let Some(count) = call_count.etag {
                etag.times(count);
            }
        }
    }

    #[test]
    fn test_desc() {
        let mock = MockSrc::with_call_count(&CallCount::zero());
        let mut src = CalendarSrc::new(mock);

        let desc = src.desc();

        assert_eq!(desc, "calendar source");
        src.src.checkpoint();
    }

    #[test]
    fn test_debug_tree() {
        let mock = MockSrc::with_call_count(&CallCount {
            debug_tree: Some(1),
            ..CallCount::zero()
        });
        let mut src = CalendarSrc::new(mock);

        let tree = src.debug_tree();

        assert_eq!(
            tree,
            TreeInfo::Wrap {
                desc: "calendar source".to_owned(),
                tp: std::any::type_name::<CalendarSrc<MockSrc>>().to_owned(),
                child: Box::new(TreeInfo::Leaf {
                    desc: "mock".to_owned(),
                    tp: std::any::type_name::<MockSrc>().to_owned()
                })
            }
        );
        src.src.checkpoint();
    }

    #[rstest]
    #[case("NYK".parse().unwrap(), 1, Ok(get_cal("NYK").unwrap()))]
    #[case("NYK|TKY".parse().unwrap(), 2, Ok(get_cal("NYK").unwrap() | get_cal("TKY").unwrap()))]
    #[case("NYK&LDN".parse().unwrap(), 2, Ok(get_cal("NYK").unwrap() & get_cal("LDN").unwrap()))]
    #[case("NYK|TKY&LDN".parse().unwrap(), 3, Ok(get_cal("NYK").unwrap() | (get_cal("TKY").unwrap() & get_cal("LDN").unwrap())))]
    #[case("XXX".parse().unwrap(), 1, Err("not found".to_owned()))]
    #[case("XXX|NYK".parse().unwrap(), 2, Err("not found".to_owned()))]
    fn test_get(
        #[case] sym: CalendarSymbol,
        #[case] num: usize,
        #[case] exp: Result<Calendar, String>,
    ) {
        let mock = MockSrc::with_call_count(&CallCount {
            get: Some(num),
            ..CallCount::zero()
        });
        let mut src = CalendarSrc::new(mock);

        let res = src.get(&sym);

        assert_eq!(res.map_err(|e| e.to_string()), exp);
        src.src.checkpoint();
    }

    #[rstest]
    #[case("NYK".parse().unwrap(), 1)]
    #[case("NYK|TKY".parse().unwrap(), 2)]
    #[case("NYK&TKY".parse().unwrap(), 2)]
    #[case("NYK|TKY&LDN".parse().unwrap(), 3)]
    fn test_etag(#[case] sym: CalendarSymbol, #[case] num: usize) {
        let mock = MockSrc::with_call_count(&CallCount {
            etag: Some(num),
            ..CallCount::zero()
        });
        let mut src = CalendarSrc::new(mock);
        let etag = src.etag(&sym).unwrap();
        src.src.checkpoint();
        src.src.setup(&CallCount {
            etag: Some(num),
            ..CallCount::zero()
        });

        let res = src.etag(&sym).unwrap();
        src.src.checkpoint();

        src.src
            .expect_etag()
            .times(num)
            .returning(|s| Ok(s.len().to_string()));
        let res2 = src.etag(&sym).unwrap();
        src.src.checkpoint();

        assert_eq!(etag, res); // etag must be the same without any change
        assert_ne!(res, res2); // etag must be different after the inner data source is changed
    }
}
