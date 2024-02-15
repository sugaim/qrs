// -----------------------------------------------------------------------------
// SingleCalendarSrc
//

use std::{
    collections::{HashMap, HashSet},
    sync::{Arc, Mutex, Weak},
};

use maplit::btreeset;

use crate::{
    chrono::CalendarSymVariant,
    datasrc::{DataSrc, Listener, NodeId, Notifier, PublisherState, StateId, TakeSnapshot, Tree},
};

use super::{Calendar, CalendarSymbol};

// -----------------------------------------------------------------------------
// _Node
//
#[derive(Debug)]
struct _Node {
    info: PublisherState,
    src_id: NodeId,
    cache: HashMap<CalendarSymbol, Calendar>,
    self_state: StateId, // invariant because this node itself does not have state
}

//
// methods
//
impl Listener for _Node {
    #[inline]
    fn id(&self) -> NodeId {
        self.info.id()
    }

    #[inline]
    fn listen(&mut self, publisher: &NodeId, state: &StateId) {
        if publisher != &self.src_id {
            return;
        }
        self.cache.clear();
        self.info.set_state(state ^ self.self_state);
    }
}

// -----------------------------------------------------------------------------
// CalendarSrc
//

/// Data source for calendars
#[derive(Debug)]
pub struct CalendarSrc<S> {
    src: S,
    node: Arc<Mutex<_Node>>,
}

//
// construction
//

impl<S: Notifier> CalendarSrc<S> {
    pub fn new(mut src: S) -> Self {
        let self_state = StateId::gen();
        let node = Arc::new(Mutex::new(_Node {
            info: PublisherState::new("calendar source"),
            src_id: src.id(),
            self_state,
            cache: HashMap::new(),
        }));
        let subsc = Arc::downgrade(&node);
        let state = src.accept_listener(subsc) ^ self_state;
        node.lock().unwrap().info.set_state(state);
        Self { src, node }
    }
}

impl<S: Clone + Notifier> Clone for CalendarSrc<S> {
    #[inline]
    fn clone(&self) -> Self {
        Self::new(self.src.clone())
    }
}

//
// methods
//
impl<S: Notifier> Notifier for CalendarSrc<S> {
    #[inline]
    fn id(&self) -> NodeId {
        self.node.lock().unwrap().info.id()
    }

    #[inline]
    fn tree(&self) -> crate::datasrc::Tree {
        let (desc, id, state) = {
            let node = self.node.lock().unwrap();
            (node.info.desc().into(), node.info.id(), node.info.state())
        };
        Tree::Branch {
            desc,
            id,
            state,
            children: btreeset! {self.src.tree()},
        }
    }

    #[inline]
    fn accept_listener(&mut self, subsc: Weak<Mutex<dyn Listener>>) -> StateId {
        self.node.lock().unwrap().info.accept_listener(subsc)
    }

    #[inline]
    fn remove_listener(&mut self, id: &NodeId) {
        self.node.lock().unwrap().info.remove_listener(id);
    }
}

impl<S: DataSrc<Key = str, Output = Calendar>> DataSrc for CalendarSrc<S> {
    type Key = CalendarSymbol;
    type Output = Calendar;
    type Err = S::Err;

    fn req(&self, key: &CalendarSymbol) -> Result<(StateId, Self::Output), Self::Err> {
        let state = self.node.lock().unwrap().info.state();
        if let Some(calendar) = self.node.lock().unwrap().cache.get(key) {
            return Ok((state, calendar.clone()));
        }

        use CalendarSymVariant::*;
        let cal = match key.dispatch() {
            Single(s) => self.src.req(s).map(|(_, c)| c)?,
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
        let mut node = self.node.lock().unwrap();
        node.cache.insert(key.clone(), cal.clone());
        Ok((state, cal))
    }
}

impl<S: TakeSnapshot<Key = str, Output = Calendar>> TakeSnapshot for CalendarSrc<S> {
    type SnapShot = CalendarSrc<S::SnapShot>;
    type SnapShotErr = S::SnapShotErr;

    fn take_snapshot<'a, It>(&self, keys: It) -> Result<Self::SnapShot, Self::SnapShotErr>
    where
        It: IntoIterator<Item = &'a CalendarSymbol>,
        CalendarSymbol: 'a,
    {
        let mut names = HashSet::new();
        for key in keys {
            key.leaves(&mut names);
        }
        let snapshot = self.src.take_snapshot(names.iter().map(|s| s.as_str()))?;
        Ok(CalendarSrc::new(snapshot))
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
    use qcore_derive::Notifier;
    use rstest::{fixture, rstest};

    use crate::{
        chrono::{Calendar, CalendarSymbol},
        datasrc::{DataSrc, Notifier, OnMemorySrc, StateId},
    };

    #[derive(Debug, Clone, Notifier)]
    #[notifier(transparent = "internal")]
    struct MockSrc {
        internal: OnMemorySrc<String, Calendar>,
    }

    impl DataSrc for MockSrc {
        type Key = str;
        type Output = Calendar;
        type Err = anyhow::Error;

        fn req(&self, key: &str) -> Result<(StateId, Calendar), anyhow::Error> {
            self.internal.req(&key.to_owned())
        }
    }

    #[fixture]
    fn single_src() -> MockSrc {
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
        MockSrc { internal: src }
    }

    #[rstest]
    fn test_req(single_src: MockSrc) {
        let nyk = single_src.req("NYK").unwrap().1;
        let tky = single_src.req("TKY").unwrap().1;
        let ldn = single_src.req("LDN").unwrap().1;

        let nyk = &nyk;
        let tky = &tky;
        let ldn = &ldn;

        let src = super::CalendarSrc::new(single_src);

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
    fn test_state_change(single_src: MockSrc) {
        let single_src = Arc::new(Mutex::new(single_src));
        let src = super::CalendarSrc::new(single_src.clone());
        let id = src.id();

        let (old, _) = src.req(&CalendarSymbol::of_single("NYK").unwrap()).unwrap();

        single_src
            .lock()
            .unwrap()
            .internal
            .remove(&"TKY".to_owned());
        let (new, _) = src.req(&CalendarSymbol::of_single("NYK").unwrap()).unwrap();
        let (cnew, _) = src.req(&CalendarSymbol::of_single("NYK").unwrap()).unwrap();

        // state is changed, but node id is not changed.
        assert_ne!(old, new);
        assert_eq!(new, cnew);
        assert_eq!(id, src.id());
    }
}
