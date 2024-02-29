// -----------------------------------------------------------------------------
// SingleCalendarSrc
//

use std::{
    collections::HashSet,
    num::NonZeroUsize,
    sync::{Arc, Mutex, Weak},
};

use lru::LruCache;
use qrs_datasrc::{
    ext::{DebugTree, TakeSnapshot},
    DataSrc, Observer, StateId, Subject,
};

use super::CalendarSymVariant;

use super::{Calendar, CalendarSymbol};

// -----------------------------------------------------------------------------
// _Node
//
#[derive(Debug)]
struct _Node {
    cache: LruCache<StateId, LruCache<CalendarSymbol, Calendar>>,
    state_shift: StateId,
    state: StateId,
    obs: Vec<Weak<Mutex<dyn Observer>>>,
}

//
// methods
//
impl Observer for _Node {
    #[inline]
    fn receive(&mut self, new_state: &StateId) {
        self.state = new_state ^ self.state_shift;
        self.obs.retain(|o| {
            let Some(o) = o.upgrade() else {
                return false;
            };
            o.lock().unwrap().receive(&self.state);
            true
        });
    }
}

// -----------------------------------------------------------------------------
// CalendarSrc
//

/// Data source for calendars
#[derive(Debug, DebugTree)]
#[debug_tree(desc_field = "desc")]
pub struct CalendarSrc<S> {
    cal_cache: NonZeroUsize,
    desc: String,
    #[debug_tree(subtree)]
    src: S,
    node: Arc<Mutex<_Node>>,
}

//
// construction
//
impl<S: DataSrc<Key = str, Output = Calendar>> CalendarSrc<S> {
    /// Create a new `CalendarSrc`
    ///
    /// - `src`: the source of calendar data
    /// - `state_cache_cap`: the capacity of the source state cache.
    /// - `cal_cache_cap`: the capacity of the source data cache.
    ///
    pub fn new(mut src: S, state_cache_cap: NonZeroUsize, cal_cache_cap: NonZeroUsize) -> Self {
        let node = Arc::new(Mutex::new(_Node {
            cache: LruCache::new(state_cache_cap),
            state_shift: StateId::gen(),
            state: StateId::gen(),
            obs: Vec::new(),
        }));
        src.reg_observer(Arc::downgrade(&node) as _);
        Self {
            cal_cache: cal_cache_cap,
            desc: "calendar source".to_owned(),
            src,
            node,
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
        Self::new(
            self.src.clone(),
            self.node.lock().unwrap().cache.cap(),
            self.cal_cache,
        )
    }
}

//
// methods
//
impl<S> Subject for CalendarSrc<S> {
    #[inline]
    fn reg_observer(&mut self, observer: Weak<Mutex<dyn Observer>>) {
        self.node.lock().unwrap().obs.push(observer);
    }

    #[inline]
    fn rm_observer(&mut self, observer: &Weak<Mutex<dyn Observer>>) {
        self.node
            .lock()
            .unwrap()
            .obs
            .retain(|o| !o.ptr_eq(observer) && o.upgrade().is_some());
    }
}

impl<S: DataSrc<Key = str, Output = Calendar>> CalendarSrc<S> {
    fn req_impl(&self, state: &StateId, key: &CalendarSymbol) -> Result<Calendar, S::Err> {
        if let Some(calendar) = self
            .node
            .lock()
            .unwrap()
            .cache
            .get_mut(state)
            .and_then(|m| m.get(key))
            .cloned()
        {
            return Ok(calendar);
        }

        use CalendarSymVariant::*;
        let cal = match key.dispatch() {
            Single(s) => self.src.req(s)?,
            AllClosed(syms) | AnyClosed(syms) => {
                let cals: Vec<Calendar> = syms
                    .iter()
                    .map(|sym| self.req_impl(state, sym))
                    .collect::<Result<_, _>>()?;
                match key.dispatch() {
                    AllClosed(_) => Calendar::of_all_closed(cals.iter()),
                    AnyClosed(_) => Calendar::of_any_closed(cals.iter()),
                    _ => unreachable!(),
                }
            }
        };
        let mut node = self.node.lock().unwrap();
        if let Some(m) = node.cache.get_mut(state) {
            m.put(key.clone(), cal.clone());
        } else {
            let mut m = LruCache::new(self.cal_cache);
            m.put(key.clone(), cal.clone());
            node.cache.put(*state, m);
        }
        Ok(cal)
    }
}

impl<S: DataSrc<Key = str, Output = Calendar>> DataSrc for CalendarSrc<S> {
    type Key = CalendarSymbol;
    type Output = Calendar;
    type Err = S::Err;

    fn req(&self, key: &CalendarSymbol) -> Result<Self::Output, Self::Err> {
        let state = self.node.lock().unwrap().state;
        self.req_impl(&state, key)
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
                CalendarSrc::new(
                    snapshot,
                    self.node.lock().unwrap().cache.cap(),
                    self.cal_cache,
                )
                .with_desc(&self.desc)
            })
    }
}

// =============================================================================
#[cfg(test)]
mod tests {
    use std::{
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
            NonZeroUsize::new(2).unwrap(),
            NonZeroUsize::new(64).unwrap(),
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
            NonZeroUsize::new(2).unwrap(),
            NonZeroUsize::new(64).unwrap(),
        );
        let record = Arc::new(Mutex::new(Vec::new()));
        let _1 = {
            let record = record.clone();
            src.on_change(move |id| {
                record.lock().unwrap().push(*id);
            })
        };

        let orig = src.req(&"NYK|TKY".try_into().unwrap()).unwrap();

        let state = src.node.lock().unwrap().state;
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
        assert_ne!(state, src.node.lock().unwrap().state);
        assert_eq!(record.lock().unwrap().len(), 1);
        assert_eq!(
            src.node.lock().unwrap().state,
            *record.lock().unwrap().last().unwrap()
        );

        let updated = src.req(&"NYK|TKY".try_into().unwrap()).unwrap();
        assert_ne!(orig, updated);
    }
}
