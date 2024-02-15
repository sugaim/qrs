use std::{
    borrow::Borrow,
    collections::HashMap,
    hash::Hash,
    sync::{Mutex, Weak},
};

use anyhow::anyhow;

use super::{
    node::DataSrc2Args, snapshot::TakeSnapshot3Args, DataSrc, DataSrc3Args, Listener, Node, NodeId,
    Notifier, PublisherState, StateId, TakeSnapshot, TakeSnapshot2Args, Tree,
};

// -----------------------------------------------------------------------------
// OnMemorySrc
//

/// A data source that stores data in a map.
///
/// Since `DataSrc` trait requires its key type as dependent type,
/// this is much rigid than [`HashMap`] itself.
///
/// Please wrap this if other key types are necessary.
#[derive(Debug)]
pub struct OnMemorySrc<K, V> {
    data: HashMap<K, V>,
    state: PublisherState,
}

//
// construction
//
impl<K, V> OnMemorySrc<K, V> {
    #[inline]
    pub fn new(desc: impl Into<String>) -> Self {
        Self::with_data(desc, HashMap::new())
    }

    #[inline]
    pub fn with_data(desc: impl Into<String>, data: HashMap<K, V>) -> Self {
        Self {
            data,
            state: PublisherState::new(desc),
        }
    }
}

impl<K: Clone, V: Clone> Clone for OnMemorySrc<K, V> {
    #[inline]
    fn clone(&self) -> Self {
        Self::with_data(self.state.desc(), self.data.clone())
    }
}

//
// methods
//
impl<K, V> Node for OnMemorySrc<K, V>
where
    K: 'static + Send + Sync,
    V: 'static + Send + Sync,
{
    #[inline]
    fn id(&self) -> NodeId {
        self.state.id()
    }
}

impl<K: 'static + Send + Sync, V: 'static + Send + Sync> Notifier for OnMemorySrc<K, V> {
    #[inline]
    fn state(&self) -> StateId {
        self.state.state()
    }

    #[inline]
    fn tree(&self) -> Tree {
        self.state.make_tree_as_leaf()
    }

    #[inline]
    fn accept_listener(&mut self, subsc: Weak<Mutex<dyn Listener>>) {
        self.state.accept_listener(subsc);
    }

    #[inline]
    fn remove_listener(&mut self, id: &NodeId) {
        self.state.remove_listener(id);
    }
}

impl<K, V> DataSrc for OnMemorySrc<K, V>
where
    K: 'static + Send + Sync + Eq + Hash,
    V: 'static + Send + Sync + Clone,
{
    type Key = K;
    type Output = V;
    type Err = anyhow::Error;

    fn req(&self, key: &K) -> Result<V, Self::Err> {
        self.data
            .get(key)
            .map(|v| v.clone())
            .ok_or_else(|| anyhow::anyhow!("key not found"))
    }
}

impl<K, V> TakeSnapshot for OnMemorySrc<K, V>
where
    K: 'static + Send + Sync + Clone + Eq + Hash,
    V: 'static + Send + Sync + Clone,
{
    type SnapShot = OnMemorySrc<K, V>;
    type SnapShotErr = Self::Err;

    fn take_snapshot<'a, It>(&self, keys: It) -> Result<Self::SnapShot, Self::Err>
    where
        It: IntoIterator<Item = &'a K>,
        K: 'a,
    {
        keys.into_iter()
            .map(|k| {
                self.data
                    .get_key_value(k)
                    .ok_or_else(|| anyhow!("key not found"))
                    .map(|(k, v)| (k.clone(), v.clone()))
            })
            .collect::<Result<HashMap<_, _>, _>>()
            .map(|data| Self::with_data(self.state.desc(), data))
    }
}

impl<K: Eq + Hash, V> OnMemorySrc<K, V> {
    #[inline]
    pub fn get<Q>(&self, key: &Q) -> Option<&V>
    where
        K: Borrow<Q>,
        Q: Eq + Hash + ?Sized,
    {
        self.data.get(key)
    }

    #[inline]
    pub fn contains_key<Q>(&self, key: &Q) -> bool
    where
        K: Borrow<Q>,
        Q: Eq + Hash + ?Sized,
    {
        self.get(key).is_some()
    }

    #[inline]
    pub fn is_empty(&self) -> bool {
        self.data.is_empty()
    }

    pub fn insert(&mut self, key: K, value: V) {
        self.data.insert(key, value);
        self.state.set_state(StateId::gen());
    }

    pub fn remove<Q>(&mut self, key: &Q) -> Option<V>
    where
        K: Borrow<Q>,
        Q: Eq + Hash + ?Sized,
    {
        self.data.remove(key).map(|v| {
            self.state.set_state(StateId::gen());
            v
        })
    }

    pub fn retain(&mut self, f: impl FnMut(&K, &mut V) -> bool) {
        let orig_len = self.data.len();
        self.data.retain(f);
        if orig_len != self.data.len() {
            self.state.set_state(StateId::gen());
        }
    }

    pub fn clear(&mut self) {
        if !self.data.is_empty() {
            self.data.clear();
            self.state.set_state(StateId::gen());
        }
    }
}

impl<K: Eq + Hash, V> Extend<(K, V)> for OnMemorySrc<K, V> {
    fn extend<T: IntoIterator<Item = (K, V)>>(&mut self, iter: T) {
        let orig_len = self.data.len();
        self.data.extend(iter);
        if orig_len != self.data.len() {
            let new_state = StateId::gen();
            self.state.set_state(new_state);
        }
    }
}

// -----------------------------------------------------------------------------
// OnMemorySrc2Args
//

/// A data source that stores data in a map.
///
/// Since `DataSrc2Args` trait requires its key types as dependent types,
/// this is much rigid than [`HashMap`] itself.
///
/// Please wrap this if other key types are necessary.
#[derive(Debug)]
pub struct OnMemorySrc2Args<K1, K2, V> {
    data: HashMap<K1, HashMap<K2, V>>,
    state: PublisherState,
}

//
// construction
//
impl<K1, K2, V> OnMemorySrc2Args<K1, K2, V> {
    #[inline]
    pub fn with_data(desc: impl Into<String>, data: HashMap<K1, HashMap<K2, V>>) -> Self {
        Self {
            data,
            state: PublisherState::new(desc),
        }
    }

    #[inline]
    pub fn new(desc: impl Into<String>) -> Self {
        Self::with_data(desc, HashMap::new())
    }
}

impl<K1: Clone, K2: Clone, V: Clone> Clone for OnMemorySrc2Args<K1, K2, V> {
    #[inline]
    fn clone(&self) -> Self {
        Self::with_data(self.state.desc(), self.data.clone())
    }
}

//
// methods
//
impl<K1, K2, V> Node for OnMemorySrc2Args<K1, K2, V>
where
    K1: 'static + Send + Sync,
    K2: 'static + Send + Sync,
    V: 'static + Send + Sync,
{
    #[inline]
    fn id(&self) -> NodeId {
        self.state.id()
    }
}

impl<K1, K2, V> Notifier for OnMemorySrc2Args<K1, K2, V>
where
    K1: 'static + Send + Sync,
    K2: 'static + Send + Sync,
    V: 'static + Send + Sync,
{
    #[inline]
    fn state(&self) -> StateId {
        self.state.state()
    }

    #[inline]
    fn tree(&self) -> Tree {
        self.state.make_tree_as_leaf()
    }

    #[inline]
    fn accept_listener(&mut self, subsc: Weak<Mutex<dyn Listener>>) {
        self.state.accept_listener(subsc);
    }

    #[inline]
    fn remove_listener(&mut self, id: &NodeId) {
        self.state.remove_listener(id);
    }
}

impl<K1, K2, V> DataSrc2Args for OnMemorySrc2Args<K1, K2, V>
where
    K1: 'static + Send + Sync + Eq + Hash,
    K2: 'static + Send + Sync + Eq + Hash,
    V: 'static + Send + Sync + Clone,
{
    type Key1 = K1;
    type Key2 = K2;
    type Output = V;
    type Err = anyhow::Error;

    fn req(&self, key1: &K1, key2: &K2) -> Result<Self::Output, Self::Err> {
        self.data
            .get(key1)
            .and_then(|m| m.get(key2))
            .map(|v| v.clone())
            .ok_or_else(|| anyhow::anyhow!("key not found"))
    }
}

impl<K1, K2, V> TakeSnapshot2Args for OnMemorySrc2Args<K1, K2, V>
where
    K1: 'static + Send + Sync + Clone + Eq + Hash,
    K2: 'static + Send + Sync + Clone + Eq + Hash,
    V: 'static + Send + Sync + Clone,
{
    type SnapShot = OnMemorySrc2Args<K1, K2, V>;
    type SnapShotErr = Self::Err;

    fn take_snapshot<'a, It>(&self, keys: It) -> Result<Self::SnapShot, Self::SnapShotErr>
    where
        It: IntoIterator<Item = (&'a K1, &'a K2)>,
        K1: 'a,
        K2: 'a,
    {
        let mut data = HashMap::new();
        let retrived = keys.into_iter().map(|(k1, k2)| {
            self.data
                .get_key_value(k1)
                .and_then(|(k1, m)| m.get_key_value(k2).map(|(k2, v)| (k1, k2, v)))
                .ok_or_else(|| anyhow!("key not found"))
        });
        for item in retrived {
            let (k1, k2, v) = item?;
            data.entry(k1.clone())
                .or_insert_with(HashMap::new)
                .insert(k2.clone(), v.clone());
        }
        Ok(Self::with_data(self.state.desc(), data))
    }
}

impl<K1: Eq + Hash, K2: Eq + Hash, V> OnMemorySrc2Args<K1, K2, V> {
    #[inline]
    pub fn get<Q1, Q2>(&self, key1: &Q1, key2: &Q2) -> Option<&V>
    where
        K1: Borrow<Q1>,
        K2: Borrow<Q2>,
        Q1: Eq + Hash + ?Sized,
        Q2: Eq + Hash + ?Sized,
    {
        self.data.get(key1).and_then(|m| m.get(key2))
    }

    #[inline]
    pub fn contains_key<Q1, Q2>(&self, key1: &Q1, key2: &Q2) -> bool
    where
        K1: Borrow<Q1>,
        K2: Borrow<Q2>,
        Q1: Eq + Hash + ?Sized,
        Q2: Eq + Hash + ?Sized,
    {
        self.get(key1, key2).is_some()
    }

    #[inline]
    pub fn is_empty(&self) -> bool {
        self.data.is_empty()
    }

    pub fn insert(&mut self, key1: K1, key2: K2, value: V) {
        self.data
            .entry(key1)
            .or_insert_with(HashMap::new)
            .insert(key2, value);
        self.state.set_state(StateId::gen());
    }

    pub fn remove<Q1, Q2>(&mut self, key1: &Q1, key2: &Q2) -> Option<V>
    where
        K1: Borrow<Q1>,
        K2: Borrow<Q2>,
        Q1: Eq + Hash + ?Sized,
        Q2: Eq + Hash + ?Sized,
    {
        match self.data.get_mut(key1) {
            None => None,
            Some(m) => {
                let res = m.remove(key2).map(|v| {
                    self.state.set_state(StateId::gen());
                    v
                });
                if m.is_empty() {
                    self.data.remove(key1);
                }
                res
            }
        }
    }

    pub fn retain(&mut self, mut f: impl FnMut(&K1, &K2, &mut V) -> bool) {
        let mut has_modified = false;
        self.data.iter_mut().for_each(|(k1, m)| {
            let orig_len = m.len();
            m.retain(|k2, v| f(k1, k2, v));
            if orig_len != m.len() {
                has_modified = true;
            }
        });
        self.data.retain(|_, m| !m.is_empty());
        if has_modified {
            self.state.set_state(StateId::gen());
        }
    }

    pub fn clear(&mut self) {
        if !self.data.is_empty() {
            self.data.clear();
            self.state.set_state(StateId::gen());
        }
    }
}

impl<K1, K2, V, It> Extend<(K1, It)> for OnMemorySrc2Args<K1, K2, V>
where
    K1: Eq + Hash,
    K2: Eq + Hash,
    It: IntoIterator<Item = (K2, V)>,
{
    fn extend<T: IntoIterator<Item = (K1, It)>>(&mut self, iter: T) {
        let mut has_modified = false;
        for (k1, m) in iter {
            let sub = self.data.entry(k1).or_insert_with(HashMap::new);
            let orig_len = sub.len();
            sub.extend(m);
            has_modified |= orig_len != sub.len();
        }
        if has_modified {
            let new_state = StateId::gen();
            self.state.set_state(new_state);
        }
    }
}

// -----------------------------------------------------------------------------
// OnMemorySrc3Args
//

/// A data source that stores data in a map.
///
/// Since `DataSrc3Args` trait requires its key types as dependent types,
/// this is much rigid than [`HashMap`] itself.
///
/// Please wrap this if other key types are necessary.
#[derive(Debug)]
pub struct OnMemorySrc3Args<K1, K2, K3, V> {
    data: HashMap<K1, HashMap<K2, HashMap<K3, V>>>,
    state: PublisherState,
}

//
// construction
//
impl<K1, K2, K3, V> OnMemorySrc3Args<K1, K2, K3, V> {
    #[inline]
    pub fn new(desc: impl Into<String>) -> Self {
        Self::with_data(desc, HashMap::new())
    }

    #[inline]
    pub fn with_data(
        desc: impl Into<String>,
        data: HashMap<K1, HashMap<K2, HashMap<K3, V>>>,
    ) -> Self {
        Self {
            data,
            state: PublisherState::new(desc),
        }
    }
}

impl<K1: Clone, K2: Clone, K3: Clone, V: Clone> Clone for OnMemorySrc3Args<K1, K2, K3, V> {
    #[inline]
    fn clone(&self) -> Self {
        Self::with_data(self.state.desc(), self.data.clone())
    }
}

//
// methods
//
impl<K1, K2, K3, V> Node for OnMemorySrc3Args<K1, K2, K3, V>
where
    K1: 'static + Send + Sync,
    K2: 'static + Send + Sync,
    K3: 'static + Send + Sync,
    V: 'static + Send + Sync,
{
    #[inline]
    fn id(&self) -> NodeId {
        self.state.id()
    }
}

impl<K1, K2, K3, V> Notifier for OnMemorySrc3Args<K1, K2, K3, V>
where
    K1: 'static + Send + Sync,
    K2: 'static + Send + Sync,
    K3: 'static + Send + Sync,
    V: 'static + Send + Sync,
{
    #[inline]
    fn state(&self) -> StateId {
        self.state.state()
    }

    #[inline]
    fn tree(&self) -> Tree {
        self.state.make_tree_as_leaf()
    }

    #[inline]
    fn accept_listener(&mut self, subsc: Weak<Mutex<dyn Listener>>) {
        self.state.accept_listener(subsc);
    }

    #[inline]
    fn remove_listener(&mut self, id: &NodeId) {
        self.state.remove_listener(id);
    }
}

impl<K1, K2, K3, V> DataSrc3Args for OnMemorySrc3Args<K1, K2, K3, V>
where
    K1: 'static + Send + Sync + Eq + Hash,
    K2: 'static + Send + Sync + Eq + Hash,
    K3: 'static + Send + Sync + Eq + Hash,
    V: 'static + Send + Sync + Clone,
{
    type Key1 = K1;
    type Key2 = K2;
    type Key3 = K3;
    type Output = V;
    type Err = anyhow::Error;

    fn req(&self, key1: &K1, key2: &K2, key3: &K3) -> Result<Self::Output, Self::Err> {
        self.data
            .get(key1)
            .and_then(|m1| m1.get(key2).and_then(|m2| m2.get(key3)))
            .map(|v| v.clone())
            .ok_or_else(|| anyhow::anyhow!("key not found"))
    }
}

impl<K1, K2, K3, V> TakeSnapshot3Args for OnMemorySrc3Args<K1, K2, K3, V>
where
    K1: 'static + Send + Sync + Clone + Eq + Hash,
    K2: 'static + Send + Sync + Clone + Eq + Hash,
    K3: 'static + Send + Sync + Clone + Eq + Hash,
    V: 'static + Send + Sync + Clone,
{
    type SnapShot = OnMemorySrc3Args<K1, K2, K3, V>;
    type SnapShotErr = Self::Err;

    fn take_snapshot<'a, It>(&self, keys: It) -> Result<Self::SnapShot, Self::SnapShotErr>
    where
        It: IntoIterator<Item = (&'a K1, &'a K2, &'a K3)>,
        K1: 'a,
        K2: 'a,
        K3: 'a,
    {
        let mut data = HashMap::new();
        let retrived = keys.into_iter().map(|(k1, k2, k3)| {
            self.data
                .get_key_value(k1)
                .and_then(|(k1, m1)| {
                    m1.get_key_value(k2)
                        .and_then(|(k2, m2)| m2.get_key_value(k3).map(|(k3, v)| (k1, k2, k3, v)))
                })
                .ok_or_else(|| anyhow!("key not found"))
        });
        for item in retrived {
            let (k1, k2, k3, v) = item?;
            data.entry(k1.clone())
                .or_insert_with(HashMap::new)
                .entry(k2.clone())
                .or_insert_with(HashMap::new)
                .insert(k3.clone(), v.clone());
        }
        Ok(Self::with_data(self.state.desc(), data))
    }
}

impl<K1: Eq + Hash, K2: Eq + Hash, K3: Eq + Hash, V> OnMemorySrc3Args<K1, K2, K3, V> {
    #[inline]
    pub fn get<Q1, Q2, Q3>(&self, key1: &Q1, key2: &Q2, key3: &Q3) -> Option<&V>
    where
        K1: Borrow<Q1>,
        K2: Borrow<Q2>,
        K3: Borrow<Q3>,
        Q1: Eq + Hash + ?Sized,
        Q2: Eq + Hash + ?Sized,
        Q3: Eq + Hash + ?Sized,
    {
        self.data
            .get(key1)
            .and_then(|m1| m1.get(key2).and_then(|m2| m2.get(key3)))
    }

    #[inline]
    pub fn contains_key<Q1, Q2, Q3>(&self, key1: &Q1, key2: &Q2, key3: &Q3) -> bool
    where
        K1: Borrow<Q1>,
        K2: Borrow<Q2>,
        K3: Borrow<Q3>,
        Q1: Eq + Hash + ?Sized,
        Q2: Eq + Hash + ?Sized,
        Q3: Eq + Hash + ?Sized,
    {
        self.get(key1, key2, key3).is_some()
    }

    #[inline]
    pub fn is_empty(&self) -> bool {
        self.data.is_empty()
    }

    pub fn insert(&mut self, key1: K1, key2: K2, key3: K3, value: V) {
        self.data
            .entry(key1)
            .or_insert_with(HashMap::new)
            .entry(key2)
            .or_insert_with(HashMap::new)
            .insert(key3, value);
        self.state.set_state(StateId::gen());
    }

    pub fn remove<Q1, Q2, Q3>(&mut self, key1: &Q1, key2: &Q2, key3: &Q3) -> Option<V>
    where
        K1: Borrow<Q1>,
        K2: Borrow<Q2>,
        K3: Borrow<Q3>,
        Q1: Eq + Hash + ?Sized,
        Q2: Eq + Hash + ?Sized,
        Q3: Eq + Hash + ?Sized,
    {
        match self.data.get_mut(key1) {
            None => None,
            Some(m1) => {
                let res = match m1.get_mut(key2) {
                    None => None,
                    Some(m2) => {
                        let res = m2.remove(key3).map(|v| {
                            self.state.set_state(StateId::gen());
                            v
                        });
                        if m2.is_empty() {
                            m1.remove(key2);
                        }
                        res
                    }
                };
                if m1.is_empty() {
                    self.data.remove(key1);
                }
                res
            }
        }
    }

    pub fn retain(&mut self, mut f: impl FnMut(&K1, &K2, &K3, &mut V) -> bool) {
        let mut has_modified = false;
        self.data.iter_mut().for_each(|(k1, m1)| {
            m1.iter_mut().for_each(|(k2, m2)| {
                let orig_len = m2.len();
                m2.retain(|k3, v| f(k1, k2, k3, v));
                if orig_len != m2.len() {
                    has_modified = true;
                }
            });
            m1.retain(|_, m2| !m2.is_empty());
        });
        if has_modified {
            self.data.retain(|_, m1| !m1.is_empty());
            self.state.set_state(StateId::gen());
        }
    }

    #[inline]
    pub fn clear(&mut self) {
        if self.data.is_empty() {
            return;
        }
        self.data.clear();
        self.state.set_state(StateId::gen());
    }
}

impl<K1, K2, K3, V, It> Extend<(K1, It)> for OnMemorySrc3Args<K1, K2, K3, V>
where
    K1: Eq + Hash,
    K2: Eq + Hash,
    K3: Eq + Hash,
    It: IntoIterator<Item = (K2, HashMap<K3, V>)>,
{
    fn extend<T: IntoIterator<Item = (K1, It)>>(&mut self, iter: T) {
        let mut has_modified = false;
        for (k1, m) in iter {
            let sub = self.data.entry(k1).or_insert_with(HashMap::new);
            let orig_len = sub.len();
            sub.extend(m);
            has_modified |= orig_len != sub.len();
        }
        if has_modified {
            let new_state = StateId::gen();
            self.state.set_state(new_state);
        }
    }
}

// =============================================================================
#[cfg(test)]
mod tests {

    use std::collections::HashMap;

    use maplit::hashmap;
    use rstest::{fixture, rstest};

    use crate::datasrc::Notifier;

    use super::{OnMemorySrc, OnMemorySrc2Args, OnMemorySrc3Args};

    #[fixture]
    fn src_1arg() -> OnMemorySrc<String, u32> {
        OnMemorySrc::with_data(
            "src",
            hashmap! {
                "a".to_string() => 1,
                "b".to_string() => 2,
                "c".to_string() => 3,
            },
        )
    }

    #[fixture]
    fn src_2args() -> OnMemorySrc2Args<String, String, u32> {
        OnMemorySrc2Args::with_data(
            "src",
            hashmap! {
                "a".to_string() => hashmap!{
                    "x".to_string() => 1,
                    "y".to_string() => 2,
                    "z".to_string() => 3,
                },
                "b".to_string() => hashmap!{
                    "x".to_string() => 4,
                    "y".to_string() => 5,
                    "z".to_string() => 6,
                },
                "c".to_string() => hashmap!{
                    "x".to_string() => 7,
                    "y".to_string() => 8,
                    "z".to_string() => 9,
                },
            },
        )
    }

    #[fixture]
    fn src_3args() -> OnMemorySrc3Args<String, String, String, u32> {
        OnMemorySrc3Args::with_data(
            "src",
            hashmap! {
                "a".to_string() => hashmap!{
                    "x".to_string() => hashmap!{
                        "i".to_string() => 1,
                        "j".to_string() => 2,
                        "k".to_string() => 3,
                    },
                    "y".to_string() => hashmap!{
                        "i".to_string() => 4,
                        "j".to_string() => 5,
                        "k".to_string() => 6,
                    },
                    "z".to_string() => hashmap!{
                        "i".to_string() => 7,
                        "j".to_string() => 8,
                        "k".to_string() => 9,
                    },
                },
                "b".to_string() => hashmap!{
                    "x".to_string() => hashmap!{
                        "i".to_string() => 10,
                        "j".to_string() => 11,
                        "k".to_string() => 12,
                    },
                    "y".to_string() => hashmap!{
                        "i".to_string() => 13,
                        "j".to_string() => 14,
                        "k".to_string() => 15,
                    },
                    "z".to_string() => hashmap!{
                        "i".to_string() => 16,
                        "j".to_string() => 17,
                        "k".to_string() => 18,
                    },
                },
                "c".to_string() => hashmap!{
                    "x".to_string() => hashmap!{
                        "i".to_string() => 19,
                        "j".to_string() => 20,
                        "k".to_string() => 21,
                    },
                    "y".to_string() => hashmap!{
                        "i".to_string() => 22,
                        "j".to_string() => 23,
                        "k".to_string() => 24,
                    },
                    "z".to_string() => hashmap!{
                        "i".to_string() => 25,
                        "j".to_string() => 26,
                        "k".to_string() => 27,
                    },
                },
            },
        )
    }

    #[rstest]
    fn test_1arg_get(src_1arg: OnMemorySrc<String, u32>) {
        let state = src_1arg.state();
        assert_eq!(src_1arg.get("a"), Some(&1));
        assert_eq!(state, src_1arg.state());
        assert_eq!(src_1arg.get("b"), Some(&2));
        assert_eq!(state, src_1arg.state());
        assert_eq!(src_1arg.get("c"), Some(&3));
        assert_eq!(state, src_1arg.state());
        assert_eq!(src_1arg.get("d"), None);
        assert_eq!(src_1arg.state(), state);
    }

    #[rstest]
    fn test_1arg_contains_key(src_1arg: OnMemorySrc<String, u32>) {
        let state = src_1arg.state();
        assert_eq!(src_1arg.contains_key("a"), true);
        assert_eq!(state, src_1arg.state());
        assert_eq!(src_1arg.contains_key("b"), true);
        assert_eq!(state, src_1arg.state());
        assert_eq!(src_1arg.contains_key("c"), true);
        assert_eq!(state, src_1arg.state());
        assert_eq!(src_1arg.contains_key("d"), false);
        assert_eq!(state, src_1arg.state());
    }

    #[rstest]
    fn test_1arg_is_empty(src_1arg: OnMemorySrc<String, u32>) {
        let state = src_1arg.state();
        assert_eq!(src_1arg.is_empty(), false);
        assert_eq!(state, src_1arg.state());
        let mut src = OnMemorySrc::new("src");
        let state = src.state();
        assert_eq!(src.is_empty(), true);
        assert_eq!(state, src.state());
        src.insert("a".to_string(), 1);
        assert_eq!(src.is_empty(), false);
        assert_ne!(state, src.state());
    }

    #[rstest]
    fn test_1arg_retain(src_1arg: OnMemorySrc<String, u32>) {
        let mut src = src_1arg;
        let state = src.state();
        src.retain(|k, v| k == "a" || *v == 3);
        assert_ne!(src.state(), state);
        assert_eq!(src.get("a"), Some(&1));
        assert_eq!(src.get("b"), None);
        assert_eq!(src.get("c"), Some(&3));

        // no change
        let state = src.state();
        src.retain(|_, _| true);
        assert_eq!(src.state(), state);
    }

    #[rstest]
    fn test_1arg_remove(src_1arg: OnMemorySrc<String, u32>) {
        let mut src = src_1arg;
        let state = src.state();
        src.remove("a");
        assert_ne!(src.state(), state);
        assert_eq!(src.get("a"), None);
        assert_eq!(src.get("b"), Some(&2));
        assert_eq!(src.get("c"), Some(&3));

        // no change
        let state = src.state();
        src.remove("d");
        assert_eq!(src.state(), state);
    }

    #[rstest]
    fn test_1arg_clear(src_1arg: OnMemorySrc<String, u32>) {
        let mut src = src_1arg;
        let state = src.state();
        src.clear();
        assert_ne!(src.state(), state);
        assert_eq!(src.is_empty(), true);

        // no change
        let state = src.state();
        src.clear();
        assert_eq!(src.state(), state);
    }

    #[rstest]
    fn test_1arg_extend(src_1arg: OnMemorySrc<String, u32>) {
        let mut src = src_1arg;
        let state = src.state();
        src.extend(vec![("d".to_string(), 4), ("e".to_string(), 5)]);
        assert_eq!(src.get("d"), Some(&4));
        assert_eq!(src.get("e"), Some(&5));
        assert_ne!(src.state(), state);

        // no change
        let state = src.state();
        src.extend(vec![]);
        assert_eq!(src.state(), state);
    }

    #[rstest]
    fn test_2args_get(src_2args: OnMemorySrc2Args<String, String, u32>) {
        let state = src_2args.state();
        assert_eq!(src_2args.get("a", "x"), Some(&1));
        assert_eq!(state, src_2args.state());
        assert_eq!(src_2args.get("b", "y"), Some(&5));
        assert_eq!(state, src_2args.state());
        assert_eq!(src_2args.get("c", "z"), Some(&9));
        assert_eq!(state, src_2args.state());
        assert_eq!(src_2args.get("d", "z"), None);
        assert_eq!(src_2args.state(), state);
    }

    #[rstest]
    fn test_2args_contains_key(src_2args: OnMemorySrc2Args<String, String, u32>) {
        let state = src_2args.state();
        assert_eq!(src_2args.contains_key("a", "x"), true);
        assert_eq!(state, src_2args.state());
        assert_eq!(src_2args.contains_key("b", "y"), true);
        assert_eq!(state, src_2args.state());
        assert_eq!(src_2args.contains_key("c", "z"), true);
        assert_eq!(state, src_2args.state());
        assert_eq!(src_2args.contains_key("d", "z"), false);
        assert_eq!(src_2args.state(), state);
    }

    #[rstest]
    fn test_2args_is_empty(src_2args: OnMemorySrc2Args<String, String, u32>) {
        let state = src_2args.state();
        assert_eq!(src_2args.is_empty(), false);
        assert_eq!(state, src_2args.state());
        let mut src = OnMemorySrc2Args::new("src");
        let state = src.state();
        assert_eq!(src.is_empty(), true);
        assert_eq!(state, src.state());
        src.insert("a".to_string(), "x".to_string(), 1);
        assert_eq!(src.is_empty(), false);
        assert_ne!(state, src.state());
    }

    #[rstest]
    fn test_2args_retain(src_2args: OnMemorySrc2Args<String, String, u32>) {
        let mut src = src_2args;
        let state = src.state();
        src.retain(|k1, k2, _| k1 == "a" || k2 == "z");
        assert_ne!(src.state(), state);
        assert_eq!(src.get("a", "x"), Some(&1));
        assert_eq!(src.get("b", "y"), None);
        assert_eq!(src.get("c", "z"), Some(&9));

        // no change
        let state = src.state();
        src.retain(|_, _, _| true);
        assert_eq!(src.state(), state);
    }

    #[rstest]
    fn test_2args_remove(src_2args: OnMemorySrc2Args<String, String, u32>) {
        let mut src = src_2args;
        let state = src.state();
        src.remove("a", "x");
        assert_ne!(src.state(), state);
        assert_eq!(src.get("a", "x"), None);
        assert_eq!(src.get("b", "y"), Some(&5));
        assert_eq!(src.get("c", "z"), Some(&9));

        // no change
        let state = src.state();
        src.remove("d", "z");
        assert_eq!(src.state(), state);
    }

    #[rstest]
    fn test_2args_clear(src_2args: OnMemorySrc2Args<String, String, u32>) {
        let mut src = src_2args;
        let state = src.state();
        src.clear();
        assert_ne!(src.state(), state);
        assert_eq!(src.is_empty(), true);

        // no change
        let state = src.state();
        src.clear();
        assert_eq!(src.state(), state);
    }

    #[rstest]
    fn test_2args_extend(src_2args: OnMemorySrc2Args<String, String, u32>) {
        let mut src = src_2args;
        let state = src.state();
        src.extend(vec![
            ("d".to_string(), hashmap! {"x".to_string() => 10}),
            ("e".to_string(), hashmap! {"y".to_string() => 11}),
        ]);
        assert_eq!(src.get("d", "x"), Some(&10));
        assert_eq!(src.get("e", "y"), Some(&11));
        assert_ne!(src.state(), state);

        // no change
        let state = src.state();
        src.extend(HashMap::<String, HashMap<String, u32>>::default());
        assert_eq!(src.state(), state);
    }

    #[rstest]
    fn test_3args_get(src_3args: OnMemorySrc3Args<String, String, String, u32>) {
        let state = src_3args.state();
        assert_eq!(src_3args.get("a", "x", "i"), Some(&1));
        assert_eq!(state, src_3args.state());
        assert_eq!(src_3args.get("b", "y", "j"), Some(&14));
        assert_eq!(state, src_3args.state());
        assert_eq!(src_3args.get("c", "z", "k"), Some(&27));
        assert_eq!(state, src_3args.state());
        assert_eq!(src_3args.get("d", "z", "k"), None);
        assert_eq!(src_3args.state(), state);
    }

    #[rstest]
    fn test_3args_contains_key(src_3args: OnMemorySrc3Args<String, String, String, u32>) {
        let state = src_3args.state();
        assert_eq!(src_3args.contains_key("a", "x", "i"), true);
        assert_eq!(state, src_3args.state());
        assert_eq!(src_3args.contains_key("b", "y", "j"), true);
        assert_eq!(state, src_3args.state());
        assert_eq!(src_3args.contains_key("c", "z", "k"), true);
        assert_eq!(state, src_3args.state());
        assert_eq!(src_3args.contains_key("d", "z", "k"), false);
        assert_eq!(src_3args.state(), state);
    }

    #[rstest]
    fn test_3args_is_empty(src_3args: OnMemorySrc3Args<String, String, String, u32>) {
        let state = src_3args.state();
        assert_eq!(src_3args.is_empty(), false);
        assert_eq!(state, src_3args.state());
        let mut src = OnMemorySrc3Args::new("src");
        let state = src.state();
        assert_eq!(src.is_empty(), true);
        assert_eq!(state, src.state());
        src.insert("a".to_string(), "x".to_string(), "i".to_string(), 1);
        assert_eq!(src.is_empty(), false);
        assert_ne!(state, src.state());
    }

    #[rstest]
    fn test_3args_retain(src_3args: OnMemorySrc3Args<String, String, String, u32>) {
        let mut src = src_3args;
        let state = src.state();
        src.retain(|k1, _, k3, _| k1 == "a" || k3 == "j");
        assert_ne!(src.state(), state);
        assert_eq!(src.get("a", "x", "i"), Some(&1));
        assert_eq!(src.get("b", "y", "j"), Some(&14));
        assert_eq!(src.get("c", "z", "k"), None);

        // no change
        let state = src.state();
        src.retain(|_, _, _, _| true);
        assert_eq!(src.state(), state);
    }

    #[rstest]
    fn test_3args_clear(src_3args: OnMemorySrc3Args<String, String, String, u32>) {
        let mut src = src_3args;
        let state = src.state();
        src.clear();
        assert_ne!(src.state(), state);
        assert_eq!(src.is_empty(), true);

        // no change
        let state = src.state();
        src.clear();
        assert_eq!(src.state(), state);
    }

    #[rstest]
    fn test_3args_remove(src_3args: OnMemorySrc3Args<String, String, String, u32>) {
        let mut src = src_3args;
        let state = src.state();
        src.remove("a", "x", "i");
        assert_ne!(src.state(), state);
        assert_eq!(src.get("a", "x", "i"), None);
        assert_eq!(src.get("b", "y", "j"), Some(&14));
        assert_eq!(src.get("c", "z", "k"), Some(&27));

        // no change
        let state = src.state();
        src.remove("d", "z", "k");
        assert_eq!(src.state(), state);
    }

    #[rstest]
    fn test_3args_extend(src_3args: OnMemorySrc3Args<String, String, String, u32>) {
        let mut src = src_3args;
        let state = src.state();
        src.extend(vec![
            (
                "d".to_string(),
                hashmap! {
                    "x".to_string() => hashmap!{"i".to_string() => 10}
                },
            ),
            (
                "e".to_string(),
                hashmap! {
                    "y".to_string() => hashmap!{"j".to_string() => 11}
                },
            ),
        ]);
        assert_eq!(src.get("d", "x", "i"), Some(&10));
        assert_eq!(src.get("e", "y", "j"), Some(&11));
        assert_ne!(src.state(), state);

        // no change
        let state = src.state();
        src.extend(HashMap::<String, HashMap<String, HashMap<String, u32>>>::default());
        assert_eq!(src.state(), state);
    }
}
