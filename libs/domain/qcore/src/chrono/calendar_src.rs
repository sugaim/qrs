// -----------------------------------------------------------------------------
// SingleCalendarSrc
//

use std::{
    collections::{HashSet, VecDeque},
    sync::{Arc, Mutex, Weak},
};

use maplit::btreeset;
use moka::sync::{Cache, CacheBuilder};
use qcore_derive::node_transparent;

use crate::{
    chrono::CalendarSymVariant,
    datasrc::{DataSrc, Node, NodeId, NodeInfo, NodeStateId, TakeSnapshot},
};

use super::{Calendar, CalendarSymbol};

// -----------------------------------------------------------------------------
// _Core
//
#[derive(Debug)]
struct _Core<S> {
    src: S,
    state_map: Mutex<VecDeque<(NodeStateId, NodeStateId)>>,
    cache: Cache<String, Calendar>,
    info: NodeInfo,
}

//
// methods
//
impl<S: Node> Node for _Core<S> {
    #[inline]
    fn id(&self) -> NodeId {
        self.info.id()
    }

    #[inline]
    fn tree(&self) -> crate::datasrc::Tree {
        self.info.make_tree_as_branch(btreeset! {self.src.tree()})
    }

    #[inline]
    fn accept_subscriber(&self, subscriber: Weak<dyn Node>) -> NodeStateId {
        self.info.accept_subscriber(subscriber)
    }

    #[inline]
    fn remove_subscriber(&self, subscriber: &NodeId) {
        self.info.remove_subscriber(&subscriber);
    }

    fn accept_state(&self, id: &NodeId, state: &NodeStateId) {
        if id != &self.src.id() {
            return;
        }
        let mut states = self.state_map.lock().unwrap();
        let state = match states.iter().rev().find(|(d, _)| d == state) {
            Some((_, upstream_state)) => *upstream_state,
            None => {
                let new = NodeStateId::gen();
                states.push_back((state.clone(), new));
                new
            }
        };
        if self.info.state() == state {
            return;
        }
        const MAX_STATE_CACHE: usize = 10;
        while MAX_STATE_CACHE < states.len() {
            let Some((_, state)) = states.pop_front() else {
                unreachable!();
            };
            let state = state.to_string();
            let _ = self
                .cache
                .invalidate_entries_if(move |key, _| key.starts_with(&state));
        }
        self.info.set_state(state);
        self.info.notify_all();
    }
}

// -----------------------------------------------------------------------------
// CalendarSrc
//

/// Data source for calendars
#[derive(Debug)]
#[node_transparent]
pub struct CalendarSrc<S>(Arc<_Core<S>>);

//
// construction
//
impl<S> Clone for CalendarSrc<S> {
    fn clone(&self) -> Self {
        Self(self.0.clone())
    }
}

impl<S: DataSrc<str, Output = Calendar>> CalendarSrc<S> {
    pub fn new(
        src: S,
        cache_builder: CacheBuilder<String, Calendar, Cache<String, Calendar>>,
    ) -> Self {
        let info = NodeInfo::new("calendar");
        let core = Arc::new(_Core {
            src,
            state_map: VecDeque::new().into(),
            cache: cache_builder.support_invalidation_closures().build(),
            info,
        });
        let subscriber = Arc::downgrade(&core);
        let downstream_state = core.src.accept_subscriber(subscriber);
        core.state_map
            .lock()
            .unwrap()
            .push_back((downstream_state, core.info.state()));
        Self(core)
    }
}

//
// methods
//
impl<S: DataSrc<str, Output = Calendar>> DataSrc<CalendarSymbol> for CalendarSrc<S> {
    type Output = Calendar;
    type Err = S::Err;

    fn req(&self, key: &CalendarSymbol) -> Result<(NodeStateId, Self::Output), Self::Err> {
        let state = self.0.info.state();
        if let Some(calendar) = self.0.cache.get(&format!("{}:{}", state, key)) {
            return Ok((state, calendar));
        }

        use CalendarSymVariant::*;
        let cal = match key.dispatch() {
            Single(s) => self.0.src.req(s).map(|(_, c)| c)?,
            AllClosed(syms) | AnyClosed(syms) => {
                let cals: Vec<Calendar> = syms
                    .iter()
                    .map(|sym| self.req(sym).map(|(_, c)| c))
                    .collect::<Result<_, _>>()?;
                match key.dispatch() {
                    AllClosed(_) => Calendar::of_all_closed(cals.iter()),
                    AnyClosed(_) => Calendar::of_any_closed(cals.iter()),
                    _ => unreachable!(),
                }
            }
        };
        self.0
            .cache
            .insert(format!("{}:{}", state, key), cal.clone());
        Ok((state, cal))
    }
}

impl<S: TakeSnapshot<str, Output = Calendar>> TakeSnapshot<CalendarSymbol> for CalendarSrc<S> {
    type SnapShot = CalendarSrc<S::SnapShot>;

    fn take_snapshot<'a, It>(&self, keys: It) -> Result<Self::SnapShot, S::Err>
    where
        It: IntoIterator<Item = &'a CalendarSymbol>,
        CalendarSymbol: 'a,
    {
        let mut names = HashSet::new();
        for key in keys {
            key.leaves(&mut names);
        }
        let snapshot = self.0.src.take_snapshot(names.iter().map(|s| s.as_str()))?;
        Ok(CalendarSrc::new(snapshot, Cache::builder()))
    }
}

// =============================================================================
#[cfg(test)]
mod tests {
    use std::str::FromStr;

    use chrono::NaiveDate;
    use rstest::{fixture, rstest};

    use crate::{
        chrono::{Calendar, CalendarSymbol},
        datasrc::{DataSrc, Node, OnMemorySrc},
    };

    #[fixture]
    fn single_src() -> OnMemorySrc<String, Calendar> {
        let mut src = OnMemorySrc::new("single calendar");
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
        src
    }

    #[rstest]
    fn test_req(single_src: OnMemorySrc<String, Calendar>) {
        let nyk = single_src.req("NYK").unwrap().1;
        let tky = single_src.req("TKY").unwrap().1;
        let ldn = single_src.req("LDN").unwrap().1;

        let nyk = &nyk;
        let tky = &tky;
        let ldn = &ldn;

        let src = super::CalendarSrc::new(single_src, moka::sync::Cache::builder());

        // single - ok
        let sym = CalendarSymbol::of_single("NYK").unwrap();
        let cal = src.req(&sym).unwrap().1;
        assert_eq!(nyk, &cal);

        // single - err
        let sym = CalendarSymbol::of_single("XXX").unwrap();
        assert!(src.req(&sym).is_err());

        // all_closed - ok
        let sym = CalendarSymbol::from_str("NYK&TKY").unwrap();
        let cal = src.req(&sym).unwrap().1;
        assert_eq!(nyk & tky, cal);

        // all_closed - err
        let sym = CalendarSymbol::from_str("NYK&XXX").unwrap();
        assert!(src.req(&sym).is_err());

        // any_closed - ok
        let sym = CalendarSymbol::from_str("NYK|TKY").unwrap();
        let cal = src.req(&sym).unwrap().1;
        assert_eq!(nyk | tky, cal);

        // any_closed - err
        let sym = CalendarSymbol::from_str("XXX|YYY").unwrap();
        assert!(src.req(&sym).is_err());

        // combined - ok
        let sym = CalendarSymbol::from_str("NYK&TKY|LDN").unwrap();
        let cal = src.req(&sym).unwrap().1;
        assert_eq!((nyk & tky) | ldn, cal);

        // combined - err
        let sym = CalendarSymbol::from_str("NYK&XXX|YYY").unwrap();
        assert!(src.req(&sym).is_err());
    }

    #[rstest]
    fn test_copy(single_src: OnMemorySrc<String, Calendar>) {
        let src = super::CalendarSrc::new(single_src.clone(), moka::sync::Cache::builder());
        let copy = src.clone();

        // node id is shared
        assert_eq!(src.id(), copy.id());

        // state id is shared
        let (id, _) = src.req(&CalendarSymbol::of_single("NYK").unwrap()).unwrap();
        let (cid, _) = copy
            .req(&CalendarSymbol::of_single("NYK").unwrap())
            .unwrap();
        assert_eq!(id, cid);
    }

    #[rstest]
    fn test_state_change(mut single_src: OnMemorySrc<String, Calendar>) {
        let src = super::CalendarSrc::new(single_src.clone(), moka::sync::Cache::builder());
        let copy = src.clone();
        let id = src.id();

        assert_eq!(id, copy.id());

        let (old, _) = src.req(&CalendarSymbol::of_single("NYK").unwrap()).unwrap();

        single_src.remove(&"TKY".to_owned());
        let (new, _) = src.req(&CalendarSymbol::of_single("NYK").unwrap()).unwrap();
        let (cnew, _) = copy
            .req(&CalendarSymbol::of_single("NYK").unwrap())
            .unwrap();

        // state is changed, but node id is not changed.
        assert_ne!(old, new);
        assert_eq!(new, cnew);
        assert_eq!(id, src.id());
        assert_eq!(id, copy.id());
    }
}
