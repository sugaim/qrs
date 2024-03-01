use std::{
    borrow::Borrow,
    hash::Hash,
    num::NonZeroUsize,
    sync::{Arc, Mutex, Weak},
};

use derivative::Derivative;
use lru::LruCache;
use uuid::Uuid;

use crate::{Observer, StateId};

// -----------------------------------------------------------------------------
// CacheSize
//
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct CacheSize {
    pub state: NonZeroUsize,
    pub value: NonZeroUsize,
}

// -----------------------------------------------------------------------------
// _Manager
//

/// _Manager receive the state change from detectors and
/// propagate the state change to observers.
#[derive(Derivative)]
#[derivative(Debug)]
struct _Manager<K, V> {
    observers: Vec<Weak<Mutex<dyn Observer>>>,
    states: Vec<Option<StateId>>,
    state_base: StateId, // immutable
    state: StateId,
    #[allow(clippy::type_complexity)]
    cache: Option<(Mutex<LruCache<StateId, LruCache<K, V>>>, NonZeroUsize)>, // cache, cache size for values
}

// -----------------------------------------------------------------------------
// _Manager
//
impl<K: 'static + Send, V: 'static + Send> _Manager<K, V> {
    #[allow(clippy::type_complexity)]
    fn new(
        i: usize,
        state: StateId,
        cache_size: Option<CacheSize>,
    ) -> (Arc<Mutex<Self>>, Vec<Arc<Mutex<dyn Observer>>>) {
        let manager = Arc::new(Mutex::new(Self {
            observers: Vec::new(),
            states: vec![None; i],
            state_base: state,
            state,
            cache: cache_size.map(|sz| (Mutex::new(LruCache::new(sz.state)), sz.value)),
        }));
        let detectors = (0..i)
            .map(|i| {
                Arc::new(Mutex::new(_Detector {
                    manager: manager.clone(),
                    i,
                })) as _
            })
            .collect();
        (manager, detectors)
    }
    fn receive(&mut self, i: usize, state: StateId) {
        self.states[i] = Some(state);
        let mut state = self.state_base;
        for s in self.states.iter().filter_map(|x| x.as_ref()) {
            state = state ^ s;
        }
        self.state = state;
        self.observers.retain(|o| {
            let Some(o) = o.upgrade() else { return false };
            o.lock().unwrap().receive(&state);
            true
        })
    }
    fn get<Q>(&self, key: &Q) -> Option<V>
    where
        K: Borrow<Q> + Eq + Hash,
        Q: Eq + Hash,
        V: Clone,
    {
        self.cache.as_ref().and_then(|cache| {
            cache
                .0
                .lock()
                .unwrap()
                .get_mut(&self.state)
                .and_then(|c| c.get(key).cloned())
        })
    }
    fn push(&self, key: K, val: V)
    where
        K: Eq + Hash,
    {
        let Some((ref cache, value_cap)) = self.cache else {
            return;
        };
        let mut cache = cache.lock().unwrap();
        match cache.get_mut(&self.state) {
            Some(c) => {
                c.put(key, val);
            }
            None => {
                let mut c = LruCache::new(value_cap);
                c.put(key, val);
                cache.put(self.state, c);
            }
        }
    }
}

// -----------------------------------------------------------------------------
// _Detector
//
struct _Detector<K, V> {
    manager: Arc<Mutex<_Manager<K, V>>>,
    i: usize,
}

//
// methods
//
impl<K: 'static + Send, V: 'static + Send> Observer for _Detector<K, V> {
    fn receive(&mut self, state: &StateId) {
        self.manager.lock().unwrap().receive(self.i, *state);
    }
}

// -----------------------------------------------------------------------------
// Node
//
#[derive(Derivative)]
#[derivative(Debug)]
pub struct PassThroughNode<K, V> {
    manager: Arc<Mutex<_Manager<K, V>>>,
    #[derivative(Debug = "ignore")]
    detectors: Vec<Arc<Mutex<dyn Observer>>>,
}

//
// construction
//
impl<K: 'static + Send, V: 'static + Send> PassThroughNode<K, V> {
    /// Create a new PassThroughNode with i detectors.
    pub fn new(i: usize, cache_size: Option<CacheSize>) -> (Self, Vec<Weak<Mutex<dyn Observer>>>) {
        let (manager, detectors) = _Manager::new(i, StateId::gen(), cache_size);
        let this = Self { manager, detectors };
        let observers = this.detectors.iter().map(Arc::downgrade).collect();
        (this, observers)
    }

    /// Create a new PassThroughNode with 1 detector.
    /// Note that when this constructor is used,
    /// detected state change will be propagated to the observers without any modification.
    pub fn state_pass_through_unary(
        cache_size: Option<CacheSize>,
    ) -> (Self, Weak<Mutex<dyn Observer>>) {
        let (manager, detectors) = _Manager::new(1, StateId(Uuid::nil()), cache_size);
        let this = Self { manager, detectors };
        let observer = Arc::downgrade(&this.detectors[0]);
        (this, observer)
    }
}

//
// methods
//
impl<K: 'static + Send, V: 'static + Send> PassThroughNode<K, V> {
    #[inline]
    pub fn reg_observer(&self, observer: Weak<Mutex<dyn Observer>>) {
        self.manager
            .lock()
            .unwrap()
            .observers
            .retain(|o| o.upgrade().is_some());
        if observer.upgrade().is_none() {
            return;
        }
        self.manager.lock().unwrap().observers.push(observer);
    }

    #[inline]
    pub fn rm_observer(&self, observer: &Weak<Mutex<dyn Observer>>) {
        self.manager
            .lock()
            .unwrap()
            .observers
            .retain(|o| !o.ptr_eq(observer) && o.upgrade().is_some());
    }

    #[inline]
    pub fn get_from_cache<Q>(&self, key: &Q) -> Option<V>
    where
        K: Borrow<Q> + Eq + Hash,
        Q: Eq + Hash,
        V: Clone,
    {
        self.manager.lock().unwrap().get(key)
    }

    #[inline]
    pub fn push_to_cache(&self, key: K, val: V)
    where
        K: Eq + Hash,
    {
        self.manager.lock().unwrap().push(key, val);
    }

    #[inline]
    pub fn is_caching(&self) -> bool {
        self.manager.lock().unwrap().cache.is_some()
    }

    #[inline]
    pub fn state(&self) -> StateId {
        self.manager.lock().unwrap().state
    }

    #[inline]
    pub fn cache_size(&self) -> Option<CacheSize> {
        let cache = &self.manager.lock().unwrap().cache;
        cache.as_ref().map(|(c, v)| CacheSize {
            state: c.lock().unwrap().cap(),
            value: *v,
        })
    }
}
