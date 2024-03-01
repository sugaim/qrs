// -----------------------------------------------------------------------------
// SingleCalendarSrc
//

use std::{
    collections::HashSet,
    sync::{Mutex, Weak},
};

use qrs_datasrc::{
    ext::{DebugTree, TakeSnapshot},
    CacheSize, DataSrc, Observer, PassThroughNode, Subject,
};

use super::CalendarSymVariant;

use super::{Calendar, CalendarSymbol};

// -----------------------------------------------------------------------------
// CalendarSrc
//

/// Data source for calendars
#[derive(Debug, DebugTree)]
#[debug_tree(desc_field = "desc")]
pub struct CalendarSrc<S> {
    node: PassThroughNode<CalendarSymbol, Calendar>,
    desc: String,
    #[debug_tree(subtree)]
    src: S,
}

//
// construction
//
impl<S: DataSrc<Key = str, Output = Calendar>> CalendarSrc<S> {
    /// Create a new `CalendarSrc`
    ///
    /// - `src`: the source of calendar data
    /// - `cache_size`: the size of the cache. If `None`, the cache is disabled.
    ///
    pub fn new(mut src: S, cache_size: Option<CacheSize>) -> Self {
        let (node, mut detectors) = PassThroughNode::new(1, cache_size);
        src.reg_observer(detectors.pop().unwrap());
        Self {
            node,
            desc: "calendar source".to_owned(),
            src,
        }
    }
}

impl<S> CalendarSrc<S> {
    /// Add a description to the source
    #[inline]
    pub fn with_desc(self, desc: impl Into<String>) -> Self {
        Self {
            desc: desc.into().to_string(),
            ..self
        }
    }
}

impl<S: Clone + DataSrc<Key = str, Output = Calendar>> Clone for CalendarSrc<S> {
    #[inline]
    fn clone(&self) -> Self {
        Self::new(self.src.clone(), self.node.cache_size()).with_desc(self.desc.clone())
    }
}

//
// methods
//
impl<S> Subject for CalendarSrc<S> {
    #[inline]
    fn reg_observer(&mut self, observer: Weak<Mutex<dyn Observer>>) {
        self.node.reg_observer(observer);
    }

    #[inline]
    fn rm_observer(&mut self, observer: &Weak<Mutex<dyn Observer>>) {
        self.node.rm_observer(observer);
    }
}

impl<S: DataSrc<Key = str, Output = Calendar>> DataSrc for CalendarSrc<S> {
    type Key = CalendarSymbol;
    type Output = Calendar;
    type Err = S::Err;

    fn req(&self, key: &CalendarSymbol) -> Result<Self::Output, Self::Err> {
        if let Some(calendar) = self.node.get_from_cache(key) {
            return Ok(calendar);
        }

        use CalendarSymVariant::*;
        let cal = match key.dispatch() {
            Single(s) => self.src.req(s)?,
            AllClosed(syms) | AnyClosed(syms) => {
                let cals: Vec<Calendar> = syms
                    .iter()
                    .map(|sym| self.req(sym))
                    .collect::<Result<_, _>>()?;
                match key.dispatch() {
                    AllClosed(_) => Calendar::of_all_closed(cals.iter()),
                    AnyClosed(_) => Calendar::of_any_closed(cals.iter()),
                    _ => unreachable!(),
                }
            }
        };
        if self.node.is_caching() {
            self.node.push_to_cache(key.clone(), cal.clone());
        }
        Ok(cal)
    }
}

impl<S: TakeSnapshot<Key = str, Output = Calendar>> TakeSnapshot for CalendarSrc<S> {
    type Snapshot = CalendarSrc<S::Snapshot>;
    type SnapshotErr = S::SnapshotErr;

    fn take_snapshot<'a, It>(&self, it: It) -> Result<Self::Snapshot, Self::SnapshotErr>
    where
        It: IntoIterator<Item = &'a Self::Key>,
        Self::Key: 'a,
    {
        let mut names = HashSet::new();
        for key in it {
            key.leaves(&mut names);
        }
        self.src
            .take_snapshot(names.iter().map(|s| s.as_str()))
            .map(|snapshot| {
                CalendarSrc::new(snapshot, self.node.cache_size()).with_desc(&self.desc)
            })
    }
}

// =============================================================================
#[cfg(test)]
mod tests {
    use std::{
        num::NonZeroUsize,
        str::FromStr,
        sync::{Arc, Mutex},
    };

    use chrono::NaiveDate;
    use qrs_datasrc::{ext::SubjectExt, on_memory::OnMemorySrc};
    use rstest::{fixture, rstest};

    use super::*;

    #[derive(Debug, Clone, DebugTree)]
    #[debug_tree(desc = "desc")]
    struct MockSrc(OnMemorySrc<String, Calendar>);

    impl Subject for MockSrc {
        #[inline]
        fn reg_observer(&mut self, observer: Weak<Mutex<dyn Observer>>) {
            self.0.reg_observer(observer);
        }

        #[inline]
        fn rm_observer(&mut self, observer: &Weak<Mutex<dyn Observer>>) {
            self.0.rm_observer(observer);
        }
    }

    impl DataSrc for MockSrc {
        type Key = str;
        type Output = Calendar;
        type Err = anyhow::Error;

        fn req(&self, key: &str) -> Result<Calendar, anyhow::Error> {
            self.0.req(&key.to_owned())
        }
    }

    #[fixture]
    fn single_src() -> MockSrc {
        let mut src = OnMemorySrc::new();
        let from = NaiveDate::from_ymd_opt(2000, 1, 1).unwrap();
        let to = NaiveDate::from_ymd_opt(2999, 12, 31).unwrap();
        src.insert(
            "NYK".to_owned(),
            Calendar::builder()
                .with_valid_period(from, to)
                .with_extra_holidays(vec![
                    NaiveDate::from_ymd_opt(2021, 1, 1).unwrap(),
                    NaiveDate::from_ymd_opt(2021, 1, 5).unwrap(),
                    NaiveDate::from_ymd_opt(2021, 1, 6).unwrap(),
                ])
                .with_extra_business_days(vec![])
                .build()
                .unwrap(),
        );
        src.insert(
            "TKY".to_owned(),
            Calendar::builder()
                .with_valid_period(from, to)
                .with_extra_holidays(vec![NaiveDate::from_ymd_opt(2021, 1, 1).unwrap()])
                .with_extra_business_days(vec![
                    NaiveDate::from_ymd_opt(2021, 1, 2).unwrap(),
                    NaiveDate::from_ymd_opt(2021, 1, 3).unwrap(),
                ])
                .build()
                .unwrap(),
        );
        src.insert(
            "LDN".to_owned(),
            Calendar::builder()
                .with_valid_period(from, to)
                .with_extra_holidays(vec![
                    NaiveDate::from_ymd_opt(2021, 1, 1).unwrap(),
                    NaiveDate::from_ymd_opt(2021, 1, 6).unwrap(),
                    NaiveDate::from_ymd_opt(2021, 1, 7).unwrap(),
                ])
                .with_extra_business_days(vec![
                    NaiveDate::from_ymd_opt(2021, 1, 2).unwrap(),
                    NaiveDate::from_ymd_opt(2021, 1, 3).unwrap(),
                ])
                .build()
                .unwrap(),
        );
        MockSrc(src)
    }

    #[rstest]
    fn test_req(single_src: MockSrc) {
        let nyk = single_src.req("NYK").unwrap();
        let tky = single_src.req("TKY").unwrap();
        let ldn = single_src.req("LDN").unwrap();

        let nyk = &nyk;
        let tky = &tky;
        let ldn = &ldn;

        let src = super::CalendarSrc::new(
            single_src,
            Some(CacheSize {
                state: NonZeroUsize::new(2).unwrap(),
                value: NonZeroUsize::new(64).unwrap(),
            }),
        );

        // single - ok
        let sym = CalendarSymbol::of_single("NYK").unwrap();
        let cal = src.req(&sym).unwrap();
        assert_eq!(nyk, &cal);

        // single - err
        let sym = CalendarSymbol::of_single("XXX").unwrap();
        assert!(src.req(&sym).is_err());

        // all_closed - ok
        let sym = CalendarSymbol::from_str("NYK&TKY").unwrap();
        let cal = src.req(&sym).unwrap();
        assert_eq!(nyk & tky, cal);

        // all_closed - err
        let sym = CalendarSymbol::from_str("NYK&XXX").unwrap();
        assert!(src.req(&sym).is_err());

        // any_closed - ok
        let sym = CalendarSymbol::from_str("NYK|TKY").unwrap();
        let cal = src.req(&sym).unwrap();
        assert_eq!(nyk | tky, cal);

        // any_closed - err
        let sym = CalendarSymbol::from_str("XXX|YYY").unwrap();
        assert!(src.req(&sym).is_err());

        // combined - ok
        let sym = CalendarSymbol::from_str("NYK&TKY|LDN").unwrap();
        let cal = src.req(&sym).unwrap();
        assert_eq!((nyk & tky) | ldn, cal);

        // combined - err
        let sym = CalendarSymbol::from_str("NYK&XXX|YYY").unwrap();
        assert!(src.req(&sym).is_err());
    }

    #[rstest]
    fn test_state_change(single_src: MockSrc) {
        let single_src = Arc::new(Mutex::new(single_src));
        let mut src = super::CalendarSrc::new(
            single_src.clone(),
            Some(CacheSize {
                state: NonZeroUsize::new(2).unwrap(),
                value: NonZeroUsize::new(64).unwrap(),
            }),
        );
        let record = Arc::new(Mutex::new(Vec::new()));
        let _1 = {
            let record = record.clone();
            src.on_change(move |id| {
                record.lock().unwrap().push(*id);
            })
        };

        let orig = src.req(&"NYK|TKY".try_into().unwrap()).unwrap();

        let state = src.node.state();
        single_src.lock().unwrap().0.insert(
            "NYK".to_owned(),
            Calendar::builder()
                .with_valid_period(
                    NaiveDate::from_ymd_opt(2000, 1, 1).unwrap(),
                    NaiveDate::from_ymd_opt(2999, 12, 31).unwrap(),
                )
                .with_extra_holidays(vec![NaiveDate::from_ymd_opt(2021, 1, 1).unwrap()])
                .with_extra_business_days(vec![])
                .build()
                .unwrap(),
        );
        assert_ne!(state, src.node.state());
        assert_eq!(record.lock().unwrap().len(), 1);
        assert_eq!(src.node.state(), *record.lock().unwrap().last().unwrap());

        let updated = src.req(&"NYK|TKY".try_into().unwrap()).unwrap();
        assert_ne!(orig, updated);
    }
}
