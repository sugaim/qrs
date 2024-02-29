use std::borrow::Borrow;
use std::collections::HashMap;
use std::hash::Hash;

use std::sync::{Mutex, Weak};

use qrs_datasrc_derive::DebugTree;

#[cfg(feature = "serde")]
use schemars::JsonSchema;
#[cfg(feature = "serde")]
use serde::{Deserialize, Serialize};

use crate::ext::{TakeSnapshot, TakeSnapshot2Args, TakeSnapshot3Args};
use crate::{DataSrc, DataSrc2Args, DataSrc3Args, Observer, StateId, Subject};

// -----------------------------------------------------------------------------
// _StateRecorder
//
#[derive(Debug)]
struct _StateRecorder {
    state: StateId,
    obs: Vec<Weak<Mutex<dyn Observer>>>,
}

//
// construction
//
impl Default for _StateRecorder {
    #[inline]
    fn default() -> Self {
        _StateRecorder {
            state: StateId::gen(),
            obs: Vec::new(),
        }
    }
}

//
// methods
//
impl _StateRecorder {
    #[inline]
    fn reg_observer(&mut self, observer: Weak<Mutex<dyn Observer>>) {
        self.obs.retain(|o| o.upgrade().is_some());
        if observer.upgrade().is_some() {
            self.obs.push(observer);
        }
    }

    #[inline]
    fn rm_observer(&mut self, observer: &Weak<Mutex<dyn Observer>>) {
        self.obs
            .retain(|o| !o.ptr_eq(observer) && o.upgrade().is_some());
    }

    #[inline]
    fn updated(&mut self) {
        self.state = StateId::gen();
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
// OnMemoryDataSource
//

/// On-memory data source
#[derive(Debug, DebugTree)]
#[cfg_attr(
    feature = "serde",
    derive(Serialize, Deserialize, JsonSchema),
    serde(bound(
        serialize = "K: Eq + Hash + Serialize, V: Serialize",
        deserialize = "K: Eq + Hash + Deserialize<'de>, V: Deserialize<'de>"
    ))
)]
#[debug_tree(_use_from_qrs_datasrc, desc_field = "desc")]
pub struct OnMemorySrc<K, V> {
    #[cfg_attr(feature = "serde", serde(rename = "description"))]
    desc: String,

    data: HashMap<K, V>,

    #[cfg_attr(feature = "serde", serde(skip, default))]
    state: _StateRecorder,
}

//
// construction
//

impl<K, V> Default for OnMemorySrc<K, V> {
    #[inline]
    fn default() -> Self {
        Self::new()
    }
}

impl<K, V> From<HashMap<K, V>> for OnMemorySrc<K, V> {
    #[inline]
    fn from(data: HashMap<K, V>) -> Self {
        OnMemorySrc {
            desc: "on memory".to_owned(),
            data,
            state: _StateRecorder::default(),
        }
    }
}

impl<K, V> FromIterator<(K, V)> for OnMemorySrc<K, V>
where
    K: Eq + Hash,
{
    #[inline]
    fn from_iter<T>(iter: T) -> Self
    where
        T: IntoIterator<Item = (K, V)>,
    {
        Self {
            desc: "on memory".to_owned(),
            data: iter.into_iter().collect(),
            state: _StateRecorder::default(),
        }
    }
}

impl<K, V> OnMemorySrc<K, V> {
    /// Create a new on-memory data source.
    #[inline]
    pub fn new() -> Self {
        OnMemorySrc {
            desc: "on memory".to_owned(),
            data: HashMap::new(),
            state: _StateRecorder::default(),
        }
    }

    /// Add a description
    #[inline]
    pub fn with_desc(self, desc: impl Into<String>) -> Self {
        OnMemorySrc {
            desc: desc.into(),
            ..self
        }
    }
}

impl<K, V> Clone for OnMemorySrc<K, V>
where
    K: Clone,
    V: Clone,
{
    #[inline]
    fn clone(&self) -> Self {
        OnMemorySrc {
            desc: self.desc.clone(),
            data: self.data.clone(),
            state: _StateRecorder::default(),
        }
    }
}

//
// methods
//
impl<K, V> Subject for OnMemorySrc<K, V> {
    #[inline]
    fn reg_observer(&mut self, observer: Weak<Mutex<dyn Observer>>) {
        self.state.reg_observer(observer);
    }

    #[inline]
    fn rm_observer(&mut self, observer: &Weak<Mutex<dyn Observer>>) {
        self.state.rm_observer(observer);
    }
}

impl<K, V> DataSrc for OnMemorySrc<K, V>
where
    K: Eq + Hash,
    V: Clone,
{
    type Key = K;
    type Output = V;
    type Err = anyhow::Error;

    #[inline]
    fn req(&self, key: &Self::Key) -> Result<Self::Output, Self::Err> {
        self.data
            .get(key)
            .cloned()
            .ok_or_else(|| anyhow::anyhow!("key not found"))
    }
}

impl<K, V> TakeSnapshot for OnMemorySrc<K, V>
where
    K: Eq + Hash + Clone,
    V: Clone,
{
    type Snapshot = Self;
    type SnapshotErr = anyhow::Error;

    fn take_snapshot<'a, It>(&self, it: It) -> Result<Self::Snapshot, Self::SnapshotErr>
    where
        It: IntoIterator<Item = &'a Self::Key>,
        Self::Key: 'a,
    {
        it.into_iter()
            .map(|k| self.req(k).map(|v| (k.clone(), v)))
            .collect::<Result<Self, _>>()
    }
}

impl<K, V> OnMemorySrc<K, V> {
    #[inline]
    pub fn inner(&self) -> &HashMap<K, V> {
        &self.data
    }

    #[inline]
    pub fn into_inner(self) -> HashMap<K, V> {
        self.data
    }

    #[inline]
    pub fn len(&self) -> usize {
        self.data.len()
    }

    #[inline]
    pub fn is_empty(&self) -> bool {
        self.data.is_empty()
    }

    #[inline]
    pub fn contains_key<Q>(&self, key: &Q) -> bool
    where
        K: Eq + Hash + Borrow<Q>,
        Q: ?Sized + Eq + Hash,
    {
        self.data.contains_key(key)
    }

    #[inline]
    pub fn get<Q>(&self, key: &Q) -> Option<&V>
    where
        K: Eq + Hash + Borrow<Q>,
        Q: ?Sized + Eq + Hash,
    {
        self.data.get(key)
    }

    /// Insert a new data and send [`Change::Update`]. See also [`HashMap::insert`].
    #[inline]
    pub fn insert(&mut self, key: K, value: V) -> Option<V>
    where
        K: Eq + Hash,
    {
        let res = self.data.insert(key, value);
        self.state.updated();
        res
    }

    /// Remove a data and send [`Change::Remove`]. See also [`HashMap::remove`].
    /// If the data source does not contain the key, this method does nothing.
    #[inline]
    pub fn remove<Q>(&mut self, key: &Q) -> Option<V>
    where
        K: Eq + Hash + Borrow<Q>,
        Q: ?Sized + Eq + Hash,
    {
        self.data.remove_entry(key).map(|(_k, v)| {
            self.state.updated();
            v
        })
    }

    /// Remove a data and send [`Change::Remove`]. See also [`HashMap::remove_entry`].
    /// If the data source does not contain the key, this method does nothing.
    #[inline]
    pub fn remove_entry<Q>(&mut self, key: &Q) -> Option<(K, V)>
    where
        K: Eq + Hash + Borrow<Q>,
        Q: ?Sized + Eq + Hash,
    {
        self.data.remove_entry(key).map(|(k, v)| {
            self.state.updated();
            (k, v)
        })
    }

    /// Clear all data and send [`Change::Clear`]. See also [`HashMap::clear`].
    /// If the data source is already empty, this method does nothing.
    #[inline]
    pub fn clear(&mut self) {
        if !self.data.is_empty() {
            self.data.clear();
            self.state.updated();
        }
    }

    /// Retain only the elements specified by the predicate and send [`Change::Remove`].
    /// See also [`HashMap::retain`].
    /// If the data source is already empty or nothing is removed, this method does nothing.
    #[inline]
    pub fn retain<F>(&mut self, f: F)
    where
        F: FnMut(&K, &mut V) -> bool,
        K: Eq + Hash,
    {
        let sz = self.data.len();
        self.data.retain(f);
        if sz != self.data.len() {
            self.state.updated();
        }
    }
}

// -----------------------------------------------------------------------------
// OnMemoryDataSource2Args
//

/// On-memory data source
#[derive(Debug, DebugTree)]
#[cfg_attr(
    feature = "serde",
    derive(Serialize, Deserialize, JsonSchema),
    serde(bound(
        serialize = "K1: Eq + Hash + Serialize, K2: Eq + Hash + Serialize, V: Serialize",
        deserialize = "K1: Eq + Hash + Deserialize<'de>, K2: Eq + Hash + Deserialize<'de>, V: Deserialize<'de>"
    ))
)]
#[debug_tree(_use_from_qrs_datasrc, desc_field = "desc")]
pub struct OnMemorySrc2Args<K1, K2, V> {
    #[cfg_attr(feature = "serde", serde(rename = "description"))]
    desc: String,

    data: HashMap<K1, HashMap<K2, V>>,

    #[cfg_attr(feature = "serde", serde(skip, default))]
    state: _StateRecorder,
}

//
// construction
//
impl<K1, K2, V> Default for OnMemorySrc2Args<K1, K2, V> {
    #[inline]
    fn default() -> Self {
        Self::new()
    }
}

impl<K1, K2, V> From<HashMap<K1, HashMap<K2, V>>> for OnMemorySrc2Args<K1, K2, V> {
    #[inline]
    fn from(mut data: HashMap<K1, HashMap<K2, V>>) -> Self {
        data.retain(|_, m| !m.is_empty());
        OnMemorySrc2Args {
            desc: "on memory".to_owned(),
            data,
            state: _StateRecorder::default(),
        }
    }
}

impl<K1, K2, V> FromIterator<(K1, HashMap<K2, V>)> for OnMemorySrc2Args<K1, K2, V>
where
    K1: Eq + Hash,
    K2: Eq + Hash,
{
    #[inline]
    fn from_iter<T>(iter: T) -> Self
    where
        T: IntoIterator<Item = (K1, HashMap<K2, V>)>,
    {
        OnMemorySrc2Args {
            desc: "on memory".to_owned(),
            data: iter.into_iter().filter(|(_, m)| !m.is_empty()).collect(),
            state: _StateRecorder::default(),
        }
    }
}

impl<K1, K2, V> OnMemorySrc2Args<K1, K2, V> {
    /// Create a new on-memory data source.
    #[inline]
    pub fn new() -> Self {
        OnMemorySrc2Args {
            desc: "on memory".to_owned(),
            data: HashMap::new(),
            state: _StateRecorder::default(),
        }
    }

    /// Add a description
    #[inline]
    pub fn with_desc(self, desc: impl Into<String>) -> Self {
        OnMemorySrc2Args {
            desc: desc.into(),
            ..self
        }
    }
}

impl<K1, K2, V> Clone for OnMemorySrc2Args<K1, K2, V>
where
    K1: Clone,
    K2: Clone,
    V: Clone,
{
    #[inline]
    fn clone(&self) -> Self {
        OnMemorySrc2Args {
            desc: self.desc.clone(),
            data: self.data.clone(),
            state: _StateRecorder::default(),
        }
    }
}

//
// methods
//
impl<K1, K2, V> Subject for OnMemorySrc2Args<K1, K2, V> {
    #[inline]
    fn reg_observer(&mut self, observer: Weak<Mutex<dyn Observer>>) {
        self.state.reg_observer(observer);
    }

    #[inline]
    fn rm_observer(&mut self, observer: &Weak<Mutex<dyn Observer>>) {
        self.state.rm_observer(observer);
    }
}

impl<K1, K2, V> DataSrc2Args for OnMemorySrc2Args<K1, K2, V>
where
    K1: 'static + Eq + Hash + Clone,
    K2: 'static + Eq + Hash + Clone,
    V: Clone,
{
    type Key1 = K1;
    type Key2 = K2;
    type Output = V;
    type Err = anyhow::Error;

    #[inline]
    fn req(&self, key1: &Self::Key1, key2: &Self::Key2) -> Result<Self::Output, Self::Err> {
        self.data
            .get(key1)
            .and_then(|m| m.get(key2))
            .cloned()
            .ok_or_else(|| anyhow::anyhow!("key not found"))
    }
}

impl<K1, K2, V> TakeSnapshot2Args for OnMemorySrc2Args<K1, K2, V>
where
    K1: 'static + Eq + Hash + Clone,
    K2: 'static + Eq + Hash + Clone,
    V: Clone,
{
    type Snapshot = Self;
    type SnapshotErr = anyhow::Error;

    fn take_snapshot<'a, It, It2>(&self, it: It) -> Result<Self::Snapshot, Self::SnapshotErr>
    where
        It: IntoIterator<Item = (&'a Self::Key1, It2)>,
        It2: IntoIterator<Item = &'a Self::Key2>,
        Self::Key1: 'a,
        Self::Key2: 'a,
    {
        it.into_iter()
            .map(|(k1, it2)| {
                self.data.get(k1).and_then(|m1| {
                    it2.into_iter()
                        .map(|k2| m1.get_key_value(k2).map(|(k, v)| (k.clone(), v.clone())))
                        .collect::<Option<HashMap<_, _>>>()
                        .map(|m| (k1.clone(), m))
                })
            })
            .collect::<Option<HashMap<_, _>>>()
            .ok_or_else(|| anyhow::anyhow!("key not found"))
            .map(Into::into)
    }
}

impl<K1, K2, V> OnMemorySrc2Args<K1, K2, V> {
    #[inline]
    pub fn inner(&self) -> &HashMap<K1, HashMap<K2, V>> {
        &self.data
    }

    #[inline]
    pub fn into_inner(self) -> HashMap<K1, HashMap<K2, V>> {
        self.data
    }

    #[inline]
    pub fn len(&self) -> usize {
        self.data.values().map(|m| m.len()).sum()
    }

    #[inline]
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    #[inline]
    pub fn contains_key<Q1, Q2>(&self, key1: &Q1, key2: &Q2) -> bool
    where
        K1: 'static + Eq + Hash + Borrow<Q1>,
        K2: 'static + Eq + Hash + Borrow<Q2>,
        Q1: ?Sized + Eq + Hash,
        Q2: ?Sized + Eq + Hash,
    {
        self.data.get(key1).map_or(false, |m| m.contains_key(key2))
    }

    /// Get data
    #[inline]
    pub fn get<Q1, Q2>(&self, key1: &Q1, key2: &Q2) -> Option<&V>
    where
        K1: 'static + Eq + Hash + Borrow<Q1>,
        K2: 'static + Eq + Hash + Borrow<Q2>,
        Q1: ?Sized + Eq + Hash,
        Q2: ?Sized + Eq + Hash,
    {
        self.data.get(key1).and_then(|m| m.get(key2))
    }

    /// Insert a new data and send [`Change::Update`]. See also [`HashMap::insert`].
    pub fn insert(&mut self, key1: K1, key2: K2, value: V) -> Option<V>
    where
        K1: Eq + Hash,
        K2: Eq + Hash,
    {
        let res = self.data.entry(key1).or_default().insert(key2, value);
        self.state.updated();
        res
    }

    /// Remove a data and send [`Change::Remove`]. See also [`HashMap::remove`].
    /// If the data source does not contain the key, this method does nothing.
    pub fn remove<Q1, Q2>(&mut self, q1: &Q1, q2: &Q2) -> Option<V>
    where
        K1: Eq + Hash + Borrow<Q1>,
        K2: Eq + Hash + Borrow<Q2>,
        Q1: ?Sized + Eq + Hash,
        Q2: ?Sized + Eq + Hash,
    {
        let Some(m) = self.data.get_mut(q1) else {
            return None;
        };
        m.remove(q2).map(|v| {
            self.state.updated();
            v
        })
    }

    /// Remove a data and send [`Change::Remove`]. See also [`HashMap::remove_entry`].
    /// If the data source does not contain the key, this method does nothing.
    pub fn remove_entry<Q1, Q2>(&mut self, key1: &Q1, key2: &Q2) -> Option<(K1, K2, V)>
    where
        K1: Eq + Hash + Borrow<Q1> + Clone,
        K2: Eq + Hash + Borrow<Q2>,
        Q1: ?Sized + Eq + Hash,
        Q2: ?Sized + Eq + Hash,
    {
        let Some((k1, mut m)) = self.data.remove_entry(key1) else {
            return None;
        };
        let Some((k2, v)) = m.remove_entry(key2) else {
            if !m.is_empty() {
                self.data.insert(k1, m);
            }
            return None;
        };
        self.state.updated();
        if m.is_empty() {
            Some((k1, k2, v))
        } else {
            self.data.insert(k1.clone(), m);
            Some((k1, k2, v))
        }
    }

    /// Clear all data and send [`Change::Clear`]. See also [`HashMap::clear`].
    /// If the data source is already empty
    #[inline]
    pub fn clear(&mut self) {
        if !self.data.is_empty() {
            self.data.clear();
            self.state.updated();
        }
    }

    /// Retain only the elements specified by the predicate and send [`Change::Remove`].
    /// See also [`HashMap::retain`].
    /// If the data source is already empty or nothing is removed, this method does nothing.
    #[inline]
    pub fn retain<F>(&mut self, mut f: F)
    where
        F: FnMut(&K1, &K2, &mut V) -> bool,
        K1: Eq + Hash,
        K2: Eq + Hash,
    {
        let mut removed = false;
        self.data.retain(|k1, m| {
            m.retain(|k2, v| {
                let rm = f(k1, k2, v);
                removed |= !rm;
                rm
            });
            !m.is_empty()
        });
        if removed {
            self.state.updated();
        }
    }
}

// -----------------------------------------------------------------------------
// OnMemorySrc3Args
//

/// On-memory data source
#[derive(Debug, DebugTree)]
#[cfg_attr(
    feature = "serde",
    derive(Serialize, Deserialize, JsonSchema),
    serde(bound(
        serialize = "K1: Eq + Hash + Serialize, K2: Eq + Hash + Serialize, K3: Eq + Hash + Serialize, V: Serialize",
        deserialize = "K1: Eq + Hash + Deserialize<'de>, K2: Eq + Hash + Deserialize<'de>, K3: Eq + Hash + Deserialize<'de>, V: Deserialize<'de>"
    ))
)]
#[debug_tree(_use_from_qrs_datasrc, desc_field = "desc")]
pub struct OnMemorySrc3Args<K1, K2, K3, V> {
    #[cfg_attr(feature = "serde", serde(rename = "description"))]
    desc: String,

    data: HashMap<K1, HashMap<K2, HashMap<K3, V>>>,

    #[cfg_attr(feature = "serde", serde(skip, default))]
    state: _StateRecorder,
}

//
// construction
//
impl<K1, K2, K3, V> Default for OnMemorySrc3Args<K1, K2, K3, V> {
    #[inline]
    fn default() -> Self {
        Self::new()
    }
}

impl<K1, K2, K3, V> From<HashMap<K1, HashMap<K2, HashMap<K3, V>>>>
    for OnMemorySrc3Args<K1, K2, K3, V>
{
    #[inline]
    fn from(data: HashMap<K1, HashMap<K2, HashMap<K3, V>>>) -> Self {
        OnMemorySrc3Args {
            desc: "on memory".to_owned(),
            data,
            state: _StateRecorder::default(),
        }
    }
}

impl<K1, K2, K3, V> FromIterator<(K1, HashMap<K2, HashMap<K3, V>>)>
    for OnMemorySrc3Args<K1, K2, K3, V>
where
    K1: Eq + Hash,
    K2: Eq + Hash,
    K3: Eq + Hash,
{
    #[inline]
    fn from_iter<T>(iter: T) -> Self
    where
        T: IntoIterator<Item = (K1, HashMap<K2, HashMap<K3, V>>)>,
    {
        iter.into_iter().collect::<HashMap<_, _>>().into()
    }
}

impl<K1, K2, K3, V> OnMemorySrc3Args<K1, K2, K3, V> {
    /// Create a new on-memory data source.
    #[inline]
    pub fn new() -> Self {
        OnMemorySrc3Args {
            desc: "on memory".to_owned(),
            data: HashMap::new(),
            state: _StateRecorder::default(),
        }
    }

    /// Add a description
    #[inline]
    pub fn with_desc(self, desc: impl Into<String>) -> Self {
        OnMemorySrc3Args {
            desc: desc.into(),
            ..self
        }
    }
}

impl<K1, K2, K3, V> Clone for OnMemorySrc3Args<K1, K2, K3, V>
where
    K1: Clone,
    K2: Clone,
    K3: Clone,
    V: Clone,
{
    #[inline]
    fn clone(&self) -> Self {
        OnMemorySrc3Args {
            desc: self.desc.clone(),
            data: self.data.clone(),
            state: _StateRecorder::default(),
        }
    }
}

//
// methods
//

impl<K1, K2, K3, V> Subject for OnMemorySrc3Args<K1, K2, K3, V> {
    #[inline]
    fn reg_observer(&mut self, observer: Weak<Mutex<dyn Observer>>) {
        self.state.reg_observer(observer);
    }

    #[inline]
    fn rm_observer(&mut self, observer: &Weak<Mutex<dyn Observer>>) {
        self.state.rm_observer(observer);
    }
}

impl<K1, K2, K3, V> DataSrc3Args for OnMemorySrc3Args<K1, K2, K3, V>
where
    K1: Eq + Hash,
    K2: Eq + Hash,
    K3: Eq + Hash,
    V: Clone,
{
    type Key1 = K1;
    type Key2 = K2;
    type Key3 = K3;
    type Output = V;
    type Err = anyhow::Error;

    #[inline]
    fn req(
        &self,
        key1: &Self::Key1,
        key2: &Self::Key2,
        key3: &Self::Key3,
    ) -> Result<Self::Output, Self::Err> {
        self.data
            .get(key1)
            .and_then(|m1| m1.get(key2))
            .and_then(|m2| m2.get(key3))
            .cloned()
            .ok_or_else(|| anyhow::anyhow!("key not found"))
    }
}

impl<K1, K2, K3, V> TakeSnapshot3Args for OnMemorySrc3Args<K1, K2, K3, V>
where
    K1: Eq + Hash + Clone,
    K2: Eq + Hash + Clone,
    K3: Eq + Hash + Clone,
    V: Clone,
{
    type Snapshot = Self;
    type SnapshotErr = anyhow::Error;

    #[inline]
    fn take_snapshot<'a, It, It2, It3>(&self, it: It) -> Result<Self::Snapshot, Self::SnapshotErr>
    where
        It: IntoIterator<Item = (&'a Self::Key1, It2)>,
        It2: IntoIterator<Item = (&'a Self::Key2, It3)>,
        It3: IntoIterator<Item = &'a Self::Key3>,
        Self::Key1: 'a,
        Self::Key2: 'a,
        Self::Key3: 'a,
    {
        it.into_iter()
            .map(|(k1, it2)| {
                self.data.get(k1).and_then(|m1| {
                    it2.into_iter()
                        .map(|(k2, it3)| {
                            m1.get(k2).and_then(|m2| {
                                it3.into_iter()
                                    .map(|k3| {
                                        m2.get_key_value(k3).map(|(k, v)| (k.clone(), v.clone()))
                                    })
                                    .collect::<Option<HashMap<_, _>>>()
                                    .map(|m| (k2.clone(), m))
                            })
                        })
                        .collect::<Option<HashMap<_, _>>>()
                        .map(|m| (k1.clone(), m))
                })
            })
            .collect::<Option<HashMap<_, _>>>()
            .ok_or_else(|| anyhow::anyhow!("key not found"))
            .map(Into::into)
    }
}

impl<K1, K2, K3, V> OnMemorySrc3Args<K1, K2, K3, V> {
    #[inline]
    pub fn inner(&self) -> &HashMap<K1, HashMap<K2, HashMap<K3, V>>> {
        &self.data
    }

    #[inline]
    pub fn into_inner(self) -> HashMap<K1, HashMap<K2, HashMap<K3, V>>> {
        self.data
    }

    #[inline]
    pub fn len(&self) -> usize {
        self.data
            .values()
            .map(|m1| m1.values().map(|m2| m2.len()).sum::<usize>())
            .sum()
    }

    #[inline]
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    #[inline]
    pub fn contains_key<Q1, Q2, Q3>(&self, key1: &Q1, key2: &Q2, key3: &Q3) -> bool
    where
        K1: Eq + Hash + Borrow<Q1>,
        K2: Eq + Hash + Borrow<Q2>,
        K3: Eq + Hash + Borrow<Q3>,
        Q1: ?Sized + Eq + Hash,
        Q2: ?Sized + Eq + Hash,
        Q3: ?Sized + Eq + Hash,
    {
        self.data
            .get(key1)
            .and_then(|m1| m1.get(key2))
            .map_or(false, |m2| m2.contains_key(key3))
    }

    /// Get data
    #[inline]
    pub fn get<Q1, Q2, Q3>(&self, key1: &Q1, key2: &Q2, key3: &Q3) -> Option<&V>
    where
        K1: Eq + Hash + Borrow<Q1>,
        K2: Eq + Hash + Borrow<Q2>,
        K3: Eq + Hash + Borrow<Q3>,
        Q1: ?Sized + Eq + Hash,
        Q2: ?Sized + Eq + Hash,
        Q3: ?Sized + Eq + Hash,
    {
        self.data
            .get(key1)
            .and_then(|m1| m1.get(key2))
            .and_then(|m2| m2.get(key3))
    }

    /// Insert a new data and send [`Change::Update`]. See also [`HashMap::insert`].
    #[inline]
    pub fn insert(&mut self, key1: K1, key2: K2, key3: K3, value: V) -> Option<V>
    where
        K1: Eq + Hash,
        K2: Eq + Hash,
        K3: Eq + Hash,
    {
        let res = self
            .data
            .entry(key1)
            .or_default()
            .entry(key2)
            .or_default()
            .insert(key3, value);
        self.state.updated();
        res
    }

    /// Remove a data and send [`Change::Remove`]. See also [`HashMap::remove`].
    /// If the data source does not contain the key, this method does nothing.
    pub fn remove<Q1, Q2, Q3>(&mut self, q1: &Q1, q2: &Q2, q3: &Q3) -> Option<V>
    where
        K1: Eq + Hash + Borrow<Q1>,
        K2: Eq + Hash + Borrow<Q2>,
        K3: Eq + Hash + Borrow<Q3>,
        Q1: ?Sized + Eq + Hash,
        Q2: ?Sized + Eq + Hash,
        Q3: ?Sized + Eq + Hash,
    {
        let Some(m1) = self.data.get_mut(q1) else {
            return None;
        };
        let Some(m2) = m1.get_mut(q2) else {
            return None;
        };
        m2.remove(q3).map(|v| {
            self.state.updated();
            v
        })
    }

    /// Remove a data and send [`Change::Remove`]. See also [`HashMap::remove_entry`].
    /// If the data source does not contain the key, this method does nothing.
    pub fn remove_entry<Q1, Q2, Q3>(
        &mut self,
        key1: &Q1,
        key2: &Q2,
        key3: &Q3,
    ) -> Option<(K1, K2, K3, V)>
    where
        K1: Eq + Hash + Borrow<Q1> + Clone,
        K2: Eq + Hash + Borrow<Q2> + Clone,
        K3: Eq + Hash + Borrow<Q3>,
        Q1: ?Sized + Eq + Hash,
        Q2: ?Sized + Eq + Hash,
        Q3: ?Sized + Eq + Hash,
    {
        let Some((k1, mut m1)) = self.data.remove_entry(key1) else {
            return None;
        };
        let Some((k2, mut m2)) = m1.remove_entry(key2) else {
            if !m1.is_empty() {
                self.data.insert(k1, m1);
            }
            return None;
        };
        let Some((k3, v)) = m2.remove_entry(key3) else {
            if !m2.is_empty() {
                m1.insert(k2, m2);
                self.data.insert(k1, m1);
            }
            return None;
        };
        self.state.updated();
        if m2.is_empty() {
            if m1.is_empty() {
                Some((k1, k2, k3, v))
            } else {
                self.data.insert(k1.clone(), m1);
                Some((k1, k2, k3, v))
            }
        } else {
            m1.insert(k2.clone(), m2);
            self.data.insert(k1.clone(), m1);
            Some((k1, k2, k3, v))
        }
    }

    /// Clear all data and send [`Change::Clear`]. See also [`HashMap::clear`].
    /// If the data source is already empty
    #[inline]
    pub fn clear(&mut self) {
        if !self.data.is_empty() {
            self.data.clear();
            self.state.updated();
        }
    }

    /// Retain only the elements specified by the predicate and send [`Change::Remove`].
    /// See also [`HashMap::retain`].
    /// If the data source is already empty or nothing is removed, this method does nothing.
    pub fn retain<F>(&mut self, mut f: F)
    where
        F: FnMut(&K1, &K2, &K3, &mut V) -> bool,
        K1: Eq + Hash,
        K2: Eq + Hash,
        K3: Eq + Hash,
    {
        let mut removed = false;
        self.data.retain(|k1, m1| {
            m1.retain(|k2, m2| {
                m2.retain(|k3, v| {
                    let rm = f(k1, k2, k3, v);
                    removed |= !rm;
                    rm
                });
                !m2.is_empty()
            });
            !m1.is_empty()
        });
        if removed {
            self.state.updated();
        }
    }
}

// =============================================================================
#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use maplit::hashmap;
    use rstest::{fixture, rstest};

    use crate::ext::SubjectExt;

    use super::*;

    mod args1 {
        use super::*;

        #[fixture]
        fn data() -> HashMap<&'static str, i32> {
            hashmap! {
                "a" => 1,
                "b" => 2,
                "c" => 3,
            }
        }

        #[test]
        fn test_default() {
            let src = OnMemorySrc::<u32, i32>::default();
            assert!(src.data.is_empty());
            assert!(src.state.obs.is_empty());

            let src2 = OnMemorySrc::<u32, i32>::default();
            assert_ne!(src.state.state, src2.state.state); // this is probabilistic
        }

        #[rstest]
        fn test_from(data: HashMap<&'static str, i32>) {
            // non-empty
            let src = OnMemorySrc::from(data.clone());
            assert_eq!(src.data, data);
            assert!(src.state.obs.is_empty());

            let src1 = OnMemorySrc::from(data);
            assert_ne!(src.state.state, src1.state.state); // this is probabilistic

            // empty
            let src = OnMemorySrc::from(HashMap::<&'static str, i32>::new());
            assert!(src.data.is_empty());
            assert!(src.state.obs.is_empty());

            let src1 = OnMemorySrc::from(HashMap::<&'static str, i32>::new());
            assert_ne!(src.state.state, src1.state.state); // this is probabilistic
        }

        #[rstest]
        fn test_from_iter(data: HashMap<&'static str, i32>) {
            // non-empty
            let src = OnMemorySrc::from_iter(data.clone());
            let src1 = OnMemorySrc::from_iter(data.clone());
            assert_eq!(src.data, data);
            assert!(src.state.obs.is_empty());
            assert_ne!(src.state.state, src1.state.state); // this is probabilistic

            // empty
            let src = OnMemorySrc::from_iter(HashMap::<&'static str, i32>::new());
            let src1 = OnMemorySrc::from_iter(HashMap::<&'static str, i32>::new());
            assert!(src.data.is_empty());
            assert!(src.state.obs.is_empty());
            assert_ne!(src.state.state, src1.state.state); // this is probabilistic
        }

        #[rstest]
        fn test_new() {
            let src = OnMemorySrc::<u32, i32>::new();
            assert!(src.data.is_empty());
            assert!(src.state.obs.is_empty());

            let src1 = OnMemorySrc::<u32, i32>::new();
            assert_ne!(src.state.state, src1.state.state); // this is probabilistic
        }

        #[rstest]
        fn test_clone(data: HashMap<&'static str, i32>) {
            let mut src = OnMemorySrc::from(data);
            let state = src.state.state;
            let record = Arc::new(Mutex::new(Vec::new()));
            let _ = {
                let record = record.clone();
                src.on_change(move |id| {
                    record.lock().unwrap().push(*id);
                })
            };
            assert_eq!(src.state.obs.len(), 1);

            // data is cloned but observers are not cloned
            let cloned = src.clone();
            assert_eq!(src.state.state, state); // state is unchanged
            assert_eq!(record.lock().unwrap().len(), 0); // clone does not send any message
            assert_eq!(src.data, cloned.data);
            assert!(cloned.state.obs.is_empty());
            assert_ne!(src.state.state, cloned.state.state); // this is probabilistic

            // empty
            let mut src = OnMemorySrc::<u32, i32>::new();
            let state = src.state.state;
            let _ = {
                let record = record.clone();
                src.on_change(move |id| {
                    record.lock().unwrap().push(*id);
                })
            };
            assert_eq!(src.state.obs.len(), 1);

            let cloned = src.clone();
            assert_eq!(src.state.state, state); // state is unchanged
            assert!(cloned.data.is_empty());
            assert_eq!(record.lock().unwrap().len(), 0); // clone does not send any message
            assert!(cloned.state.obs.is_empty());
            assert_ne!(src.state.state, cloned.state.state); // this is probabilistic
        }

        #[rstest]
        fn test_req(data: HashMap<&'static str, i32>) {
            // non-empty
            let mut src = OnMemorySrc::from(data);
            let state = src.state.state;
            let record = Arc::new(Mutex::new(Vec::new()));
            let _ = {
                let record = record.clone();
                src.on_change(move |id| {
                    record.lock().unwrap().push(*id);
                })
            };
            assert_eq!(src.state.obs.len(), 1);

            // ok
            assert_eq!(src.req(&"a").unwrap(), 1);
            assert_eq!(src.state.state, state); // state is unchanged
            assert_eq!(record.lock().unwrap().len(), 0); // req method does not send any message
            assert_eq!(src.req(&"b").unwrap(), 2);
            assert_eq!(src.state.state, state); // state is unchanged
            assert_eq!(record.lock().unwrap().len(), 0); // req method does not send any message
            assert_eq!(src.req(&"c").unwrap(), 3);
            assert_eq!(src.state.state, state); // state is unchanged
            assert_eq!(record.lock().unwrap().len(), 0); // req method does not send any message

            // error
            assert!(src.req(&"d").is_err());
            assert_eq!(src.state.state, state); // state is unchanged
            assert_eq!(record.lock().unwrap().len(), 0); // req method does not send any message

            // empty
            let mut src = OnMemorySrc::<&'static str, i32>::new();
            let state = src.state.state;
            let record = Arc::new(Mutex::new(Vec::new()));
            let _ = {
                let record = record.clone();
                src.on_change(move |id| {
                    record.lock().unwrap().push(*id);
                })
            };

            assert!(src.req(&"a").is_err());
            assert_eq!(src.state.state, state); // state is unchanged
            assert_eq!(record.lock().unwrap().len(), 0); // req method does not send any message
            assert!(src.req(&"b").is_err());
            assert_eq!(src.state.state, state); // state is unchanged
            assert_eq!(record.lock().unwrap().len(), 0); // req method does not send any message
        }

        #[rstest]
        fn test_take_snapshot(data: HashMap<&'static str, i32>) {
            // non-empty
            let mut src = OnMemorySrc::from(data.clone());
            let state = src.state.state;
            let record = Arc::new(Mutex::new(Vec::new()));
            let _ = {
                let record = record.clone();
                src.on_change(move |id| {
                    record.lock().unwrap().push(*id);
                })
            };
            assert_eq!(src.state.obs.len(), 1);

            // ok
            let snapshot = src.take_snapshot(&[]).unwrap();
            assert_eq!(src.state.state, state); // state is unchanged
            assert_ne!(snapshot.state.state, state); // this is probabilistic
            assert_eq!(record.lock().unwrap().len(), 0); // take_snapshot method does not send any message
            assert!(snapshot.data.is_empty());
            assert_eq!(snapshot.state.obs.len(), 0);

            let snapshot = src.take_snapshot(&["a", "c"]).unwrap();
            assert_eq!(src.state.state, state); // state is unchanged
            assert_ne!(snapshot.state.state, state); // this is probabilistic
            assert_eq!(record.lock().unwrap().len(), 0); // take_snapshot method does not send any message
            assert_eq!(snapshot.data, hashmap! {"a" => 1, "c" => 3});
            assert_eq!(snapshot.state.obs.len(), 0);

            // error
            assert!(src.take_snapshot(&["a", "d"]).is_err());
            assert_eq!(src.state.state, state); // state is unchanged
            assert_eq!(record.lock().unwrap().len(), 0); // take_snapshot method does not send any message

            // empty
            let mut src = OnMemorySrc::<&'static str, i32>::new();
            let state = src.state.state;
            let _ = src.on_change(|id| {
                println!("id: {}", id);
            });
            assert_eq!(src.state.obs.len(), 1);

            // ok
            let snapshot = src.take_snapshot(&[]).unwrap();
            assert_eq!(src.state.state, state); // state is unchanged
            assert_eq!(record.lock().unwrap().len(), 0); // take_snapshot method does not send any message
            assert!(snapshot.data.is_empty());
            assert_eq!(snapshot.state.obs.len(), 0);

            // error
            assert!(src.take_snapshot(&["a", "c"]).is_err());
            assert_eq!(src.state.state, state); // state is unchanged
            assert_eq!(record.lock().unwrap().len(), 0); // take_snapshot method does not send any message
        }

        #[rstest]
        fn test_inner(data: HashMap<&'static str, i32>) {
            // non-empty
            let mut src = OnMemorySrc::from(data.clone());
            let state = src.state.state;
            let record = Arc::new(Mutex::new(Vec::new()));
            let _ = {
                let record = record.clone();
                src.on_change(move |id| {
                    record.lock().unwrap().push(*id);
                })
            };
            assert_eq!(src.state.obs.len(), 1);

            assert_eq!(src.inner(), &data);
            assert_eq!(src.state.state, state); // state is unchanged
            assert_eq!(record.lock().unwrap().len(), 0); // inner method does not send any message

            // empty
            let mut src = OnMemorySrc::<&'static str, i32>::new();
            let state = src.state.state;
            let _ = {
                let record = record.clone();
                src.on_change(move |id| {
                    record.lock().unwrap().push(*id);
                })
            };
            assert!(src.inner().is_empty());
            assert_eq!(src.state.state, state); // state is unchanged
            assert_eq!(record.lock().unwrap().len(), 0); // inner method does not send any message
        }

        #[rstest]
        fn test_into_inner(data: HashMap<&'static str, i32>) {
            // non-empty
            let mut src = OnMemorySrc::from(data.clone());
            let record = Arc::new(Mutex::new(Vec::new()));
            let _ = {
                let record = record.clone();
                src.on_change(move |id| {
                    record.lock().unwrap().push(*id);
                })
            };
            assert_eq!(src.state.obs.len(), 1);
            assert_eq!(src.into_inner(), data);
            assert_eq!(record.lock().unwrap().len(), 0); // into_inner method does not send any message

            // empty
            let mut src = OnMemorySrc::<&'static str, i32>::new();
            let _ = {
                let record = record.clone();
                src.on_change(move |id| {
                    record.lock().unwrap().push(*id);
                })
            };
            assert_eq!(src.state.obs.len(), 1);

            assert!(src.into_inner().is_empty());
            assert_eq!(record.lock().unwrap().len(), 0); // into_inner method does not send any message
        }

        #[rstest]
        fn test_len(data: HashMap<&'static str, i32>) {
            // non-empty
            let mut src = OnMemorySrc::from(data);
            let state = src.state.state;
            let record = Arc::new(Mutex::new(Vec::new()));
            let _ = {
                let record = record.clone();
                src.on_change(move |id| {
                    record.lock().unwrap().push(*id);
                })
            };
            assert_eq!(src.state.obs.len(), 1);

            assert_eq!(src.len(), 3);
            assert_eq!(src.state.state, state); // state is unchanged
            assert_eq!(record.lock().unwrap().len(), 0); // len method does not send any message

            // empty
            let mut src = OnMemorySrc::<&'static str, i32>::new();
            let state = src.state.state;
            let _ = {
                let record = record.clone();
                src.on_change(move |id| {
                    record.lock().unwrap().push(*id);
                })
            };
            assert_eq!(src.state.obs.len(), 1);

            assert_eq!(src.len(), 0);
            assert_eq!(src.state.state, state); // state is unchanged
            assert_eq!(record.lock().unwrap().len(), 0); // len method does not send any message
        }

        #[rstest]
        fn test_is_empty(data: HashMap<&'static str, i32>) {
            // non-empty
            let mut src = OnMemorySrc::from(data);
            let state = src.state.state;
            let record = Arc::new(Mutex::new(Vec::new()));
            let _ = {
                let record = record.clone();
                src.on_change(move |id| {
                    record.lock().unwrap().push(*id);
                })
            };
            assert_eq!(src.state.obs.len(), 1);

            assert!(!src.is_empty());
            assert_eq!(src.state.state, state); // state is unchanged
            assert_eq!(record.lock().unwrap().len(), 0); // is_empty method does not send any message

            // empty
            let mut src = OnMemorySrc::<&'static str, i32>::new();
            let state = src.state.state;
            let _ = {
                let record = record.clone();
                src.on_change(move |id| {
                    record.lock().unwrap().push(*id);
                })
            };
            assert_eq!(src.state.obs.len(), 1);

            assert!(src.is_empty());
            assert_eq!(src.state.state, state); // state is unchanged
            assert_eq!(record.lock().unwrap().len(), 0); // is_empty method does not send any message
        }

        #[rstest]
        fn test_contains_key(data: HashMap<&'static str, i32>) {
            // non-empty
            let mut src = OnMemorySrc::from(data);
            let state = src.state.state;
            let record = Arc::new(Mutex::new(Vec::new()));
            let _ = {
                let record = record.clone();
                src.on_change(move |id| {
                    record.lock().unwrap().push(*id);
                })
            };
            assert_eq!(src.state.obs.len(), 1);

            assert!(src.contains_key(&"a"));
            assert_eq!(src.state.state, state); // state is unchanged
            assert_eq!(record.lock().unwrap().len(), 0); // contains_key method does not send any message
            assert!(src.contains_key(&"b"));
            assert_eq!(src.state.state, state); // state is unchanged
            assert_eq!(record.lock().unwrap().len(), 0); // contains_key method does not send any message
            assert!(src.contains_key(&"c"));
            assert_eq!(src.state.state, state); // state is unchanged
            assert_eq!(record.lock().unwrap().len(), 0); // contains_key method does not send any message
            assert!(!src.contains_key(&"d"));
            assert_eq!(src.state.state, state); // state is unchanged
            assert_eq!(record.lock().unwrap().len(), 0); // contains_key method does not send any message

            // empty
            let mut src = OnMemorySrc::<&'static str, i32>::new();
            let state = src.state.state;
            let _ = {
                let record = record.clone();
                src.on_change(move |id| {
                    record.lock().unwrap().push(*id);
                })
            };
            assert_eq!(src.state.obs.len(), 1);

            assert!(!src.contains_key(&"a"));
            assert_eq!(src.state.state, state); // state is unchanged
            assert_eq!(record.lock().unwrap().len(), 0); // contains_key method does not send any message
            assert!(!src.contains_key(&"b"));
            assert_eq!(src.state.state, state); // state is unchanged
            assert_eq!(record.lock().unwrap().len(), 0); // contains_key method does not send any message
        }

        #[rstest]
        fn test_get(data: HashMap<&'static str, i32>) {
            // non-empty
            let mut src = OnMemorySrc::from(data);
            let state = src.state.state;
            let record = Arc::new(Mutex::new(Vec::new()));
            let _ = {
                let record = record.clone();
                src.on_change(move |id| {
                    record.lock().unwrap().push(*id);
                })
            };
            assert_eq!(src.state.obs.len(), 1);

            assert_eq!(src.get(&"a"), Some(&1));
            assert_eq!(src.state.state, state); // state is unchanged
            assert_eq!(record.lock().unwrap().len(), 0); // get method does not send any message
            assert_eq!(src.get(&"b"), Some(&2));
            assert_eq!(src.state.state, state); // state is unchanged
            assert_eq!(record.lock().unwrap().len(), 0); // get method does not send any message
            assert_eq!(src.get(&"c"), Some(&3));
            assert_eq!(src.state.state, state); // state is unchanged
            assert_eq!(record.lock().unwrap().len(), 0); // get method does not send any message
            assert_eq!(src.get(&"d"), None);
            assert_eq!(src.state.state, state); // state is unchanged
            assert_eq!(record.lock().unwrap().len(), 0); // get method does not send any message

            // empty
            let mut src = OnMemorySrc::<&'static str, i32>::new();
            let state = src.state.state;
            let _ = {
                let record = record.clone();
                src.on_change(move |id| {
                    record.lock().unwrap().push(*id);
                })
            };
            assert_eq!(src.state.obs.len(), 1);

            assert_eq!(src.get(&"a"), None);
            assert_eq!(src.state.state, state); // state is unchanged
            assert_eq!(record.lock().unwrap().len(), 0); // get method does not send any message
            assert_eq!(src.get(&"b"), None);
            assert_eq!(src.state.state, state); // state is unchanged
            assert_eq!(record.lock().unwrap().len(), 0); // get method does not send any message
        }

        #[rstest]
        fn test_insert(data: HashMap<&'static str, i32>) {
            let record1 = Arc::new(Mutex::new(Vec::new()));
            let record2 = Arc::new(Mutex::new(Vec::new()));

            // non-empty
            let mut src = OnMemorySrc::from(data.clone());
            let mut state = src.state.state;
            let _1 = {
                let record1 = record1.clone();
                src.on_change(move |id| {
                    record1.lock().unwrap().push(*id);
                })
            };
            let _2 = {
                let record2 = record2.clone();
                src.on_change(move |_| {
                    let mut record = record2.lock().unwrap();
                    let len = record.len();
                    record.push(format!("inserted{}", len));
                })
            };
            assert_eq!(src.state.obs.len(), 2);

            assert_eq!(src.insert("a", 10), Some(1));
            assert_ne!(src.state.state, state); // state is changed
            assert_eq!(record1.lock().unwrap().len(), 1); // insert method sends a message
            assert_eq!(record1.lock().unwrap()[0], src.state.state);
            assert_eq!(record2.lock().unwrap().len(), 1); // insert method sends a message
            assert_eq!(record2.lock().unwrap()[0], "inserted0");
            state = src.state.state;

            assert_eq!(src.insert("b", 20), Some(2));
            assert_ne!(src.state.state, state); // state is changed
            assert_eq!(record1.lock().unwrap().len(), 2); // insert method sends a message
            assert_eq!(record1.lock().unwrap()[1], src.state.state);
            assert_eq!(record2.lock().unwrap().len(), 2); // insert method sends a message
            assert_eq!(record2.lock().unwrap()[1], "inserted1");

            assert_eq!(src.insert("c", 30), Some(3));
            assert_ne!(src.state.state, state); // state is changed
            assert_eq!(record1.lock().unwrap().len(), 3); // insert method sends a message
            assert_eq!(record1.lock().unwrap()[2], src.state.state);
            assert_eq!(record2.lock().unwrap().len(), 3); // insert method sends a message
            assert_eq!(record2.lock().unwrap()[2], "inserted2");

            assert_eq!(src.insert("d", 40), None);
            assert_ne!(src.state.state, state); // state is changed
            assert_eq!(record1.lock().unwrap().len(), 4); // insert method sends a message
            assert_eq!(record1.lock().unwrap()[3], src.state.state);
            assert_eq!(record2.lock().unwrap().len(), 4); // insert method sends a message
            assert_eq!(record2.lock().unwrap()[3], "inserted3");
            assert_eq!(
                src.data,
                hashmap! {"a" => 10, "b" => 20, "c" => 30, "d" => 40}
            );

            // empty
            let mut src = OnMemorySrc::<&'static str, i32>::new();
            let state = src.state.state;
            let _1 = {
                let record1 = record1.clone();
                record1.lock().unwrap().clear();
                src.on_change(move |id| {
                    record1.lock().unwrap().push(*id);
                })
            };
            let _2 = {
                let record2 = record2.clone();
                record2.lock().unwrap().clear();
                src.on_change(move |_| {
                    let mut record = record2.lock().unwrap();
                    let len = record.len();
                    record.push(format!("inserted{}", len));
                })
            };

            assert_eq!(src.insert("x", 10), None);
            assert_ne!(src.state.state, state); // state is changed
            assert_eq!(record1.lock().unwrap().len(), 1); // insert method sends a message
            assert_eq!(record1.lock().unwrap()[0], src.state.state);
            assert_eq!(record2.lock().unwrap().len(), 1); // insert method sends a message
            assert_eq!(record2.lock().unwrap()[0], "inserted0");

            assert_eq!(src.data, hashmap! {"x" => 10});
        }

        #[rstest]
        fn test_remove(data: HashMap<&'static str, i32>) {
            let record1 = Arc::new(Mutex::new(Vec::new()));
            let record2 = Arc::new(Mutex::new(Vec::new()));

            // non-empty
            let mut src = OnMemorySrc::from(data.clone());
            let mut state = src.state.state;
            let _1 = {
                let record1 = record1.clone();
                src.on_change(move |id| {
                    record1.lock().unwrap().push(*id);
                })
            };
            let _2 = {
                let record2 = record2.clone();
                src.on_change(move |_| {
                    let mut record = record2.lock().unwrap();
                    let len = record.len();
                    record.push(format!("removed{}", len));
                })
            };
            assert_eq!(src.state.obs.len(), 2);

            assert_eq!(src.remove(&"a"), Some(1));
            assert_ne!(src.state.state, state); // state is changed
            assert_eq!(record1.lock().unwrap().len(), 1); // remove method sends a message
            assert_eq!(record1.lock().unwrap()[0], src.state.state);
            assert_eq!(record2.lock().unwrap().len(), 1); // remove method sends a message
            assert_eq!(record2.lock().unwrap()[0], "removed0");
            state = src.state.state;

            assert_eq!(src.remove(&"c"), Some(3));
            assert_ne!(src.state.state, state); // state is changed
            assert_eq!(record1.lock().unwrap().len(), 2); // remove method sends a message
            assert_eq!(record1.lock().unwrap()[1], src.state.state);
            assert_eq!(record2.lock().unwrap().len(), 2); // remove method sends
            assert_eq!(record2.lock().unwrap()[1], "removed1");
            state = src.state.state;

            assert_eq!(src.remove(&"d"), None);
            assert_eq!(src.state.state, state); // state is unchanged
            assert_eq!(record1.lock().unwrap().len(), 2); // remove method does not send any message
            assert_eq!(record2.lock().unwrap().len(), 2); // remove method does not send any message
            assert_eq!(src.data, hashmap! {"b" => 2});
        }

        #[rstest]
        fn test_remove_entry(data: HashMap<&'static str, i32>) {
            let record1 = Arc::new(Mutex::new(Vec::new()));
            let record2 = Arc::new(Mutex::new(Vec::new()));

            // non-empty
            let mut src = OnMemorySrc::from(data.clone());
            let mut state = src.state.state;
            let _1 = {
                let record1 = record1.clone();
                src.on_change(move |id| {
                    record1.lock().unwrap().push(*id);
                })
            };
            let _2 = {
                let record2 = record2.clone();
                src.on_change(move |_| {
                    let mut record = record2.lock().unwrap();
                    let len = record.len();
                    record.push(format!("removed{}", len));
                })
            };
            assert_eq!(src.state.obs.len(), 2);

            assert_eq!(src.remove_entry(&"a"), Some(("a", 1)));
            assert_ne!(src.state.state, state); // state is changed
            assert_eq!(record1.lock().unwrap().len(), 1); // remove_entry method sends a message
            assert_eq!(record1.lock().unwrap()[0], src.state.state);
            assert_eq!(record2.lock().unwrap().len(), 1); // remove_entry method sends a message
            assert_eq!(record2.lock().unwrap()[0], "removed0");
            state = src.state.state;

            assert_eq!(src.remove_entry(&"c"), Some(("c", 3)));
            assert_ne!(src.state.state, state); // state is changed
            assert_eq!(record1.lock().unwrap().len(), 2); // remove_entry method sends a message
            assert_eq!(record1.lock().unwrap()[1], src.state.state);
            assert_eq!(record2.lock().unwrap().len(), 2); // remove_entry method sends a message
            assert_eq!(record2.lock().unwrap()[1], "removed1");
            state = src.state.state;

            assert_eq!(src.remove_entry(&"d"), None);
            assert_eq!(src.state.state, state); // state is unchanged
            assert_eq!(record1.lock().unwrap().len(), 2); // remove_entry method does not send any message
            assert_eq!(record2.lock().unwrap().len(), 2); // remove_entry method does not send any message
            assert_eq!(src.data, hashmap! {"b" => 2});
        }

        #[rstest]
        fn test_clear(data: HashMap<&'static str, i32>) {
            let record1 = Arc::new(Mutex::new(Vec::new()));
            let record2 = Arc::new(Mutex::new(Vec::new()));

            // non-empty
            let mut src = OnMemorySrc::from(data.clone());
            let state = src.state.state;
            let _1 = {
                let record1 = record1.clone();
                src.on_change(move |id| {
                    record1.lock().unwrap().push(*id);
                })
            };
            let _2 = {
                let record2 = record2.clone();
                src.on_change(move |_| {
                    let mut record = record2.lock().unwrap();
                    let len = record.len();
                    record.push(format!("cleared{}", len));
                })
            };
            assert_eq!(src.state.obs.len(), 2);

            src.clear();
            assert_ne!(src.state.state, state); // state is changed
            assert_eq!(record1.lock().unwrap().len(), 1); // clear method sends a message
            assert_eq!(record1.lock().unwrap()[0], src.state.state);
            assert_eq!(record2.lock().unwrap().len(), 1); // clear method sends a message
            assert_eq!(record2.lock().unwrap()[0], "cleared0");
            assert!(src.data.is_empty());
            let state = src.state.state;

            src.clear();
            assert_eq!(src.state.state, state); // state is unchanged
            assert_eq!(record1.lock().unwrap().len(), 1); // clear method does not send any message
            assert_eq!(record2.lock().unwrap().len(), 1); // clear method does not send any message
            assert!(src.data.is_empty());

            // empty
            let mut src = OnMemorySrc::<&'static str, i32>::new();
            let state = src.state.state;
            let _1 = {
                let record1 = record1.clone();
                record1.lock().unwrap().clear();
                src.on_change(move |id| {
                    record1.lock().unwrap().push(*id);
                })
            };
            let _2 = {
                let record2 = record2.clone();
                record2.lock().unwrap().clear();
                src.on_change(move |_| {
                    let mut record = record2.lock().unwrap();
                    let len = record.len();
                    record.push(format!("cleared{}", len));
                })
            };

            src.clear();
            assert_eq!(src.state.state, state); // state is unchanged
            assert_eq!(record1.lock().unwrap().len(), 0); // clear method does not send any message
            assert_eq!(record2.lock().unwrap().len(), 0); // clear method does not send any message
            assert!(src.data.is_empty());
        }

        #[rstest]
        fn test_retain(data: HashMap<&'static str, i32>) {
            let record1 = Arc::new(Mutex::new(Vec::new()));
            let record2 = Arc::new(Mutex::new(Vec::new()));

            // non-empty
            let mut src = OnMemorySrc::from(data.clone());
            let state = src.state.state;
            let _1 = {
                let record1 = record1.clone();
                src.on_change(move |id| {
                    record1.lock().unwrap().push(*id);
                })
            };
            let _2 = {
                let record2 = record2.clone();
                src.on_change(move |_| {
                    let mut record = record2.lock().unwrap();
                    let len = record.len();
                    record.push(format!("retained{}", len));
                })
            };
            assert_eq!(src.state.obs.len(), 2);

            src.retain(|_, v| *v % 2 != 0);
            assert_ne!(src.state.state, state); // state is changed
            assert_eq!(record1.lock().unwrap().len(), 1); // retain method sends a message
            assert_eq!(record1.lock().unwrap()[0], src.state.state);
            assert_eq!(record2.lock().unwrap().len(), 1); // retain method sends a message
            assert_eq!(record2.lock().unwrap()[0], "retained0");
            assert_eq!(src.data, hashmap! {"a" => 1, "c" => 3});
            let state = src.state.state;

            src.retain(|c, _| c != &"b"); // different function, but equivalent to the previous one
            assert_eq!(src.state.state, state); // state is unchanged
            assert_eq!(record1.lock().unwrap().len(), 1); // retain method does not send any message
            assert_eq!(record2.lock().unwrap().len(), 1); // retain method does not send any message
            assert_eq!(src.data, hashmap! {"a" => 1, "c" => 3});

            src.retain(|_, v| *v % 2 == 0);
            assert_ne!(src.state.state, state); // state is changed
            assert_eq!(record1.lock().unwrap().len(), 2); // retain method sends a message
            assert_eq!(record1.lock().unwrap()[1], src.state.state);
            assert_eq!(record2.lock().unwrap().len(), 2); // retain method sends a message
            assert_eq!(record2.lock().unwrap()[1], "retained1");
            assert!(src.data.is_empty());

            // empty
            let mut src = OnMemorySrc::<&'static str, i32>::new();
            let state = src.state.state;
            let _1 = {
                let record1 = record1.clone();
                record1.lock().unwrap().clear();
                src.on_change(move |id| {
                    record1.lock().unwrap().push(*id);
                })
            };
            let _2 = {
                let record2 = record2.clone();
                record2.lock().unwrap().clear();
                src.on_change(move |_| {
                    let mut record = record2.lock().unwrap();
                    let len = record.len();
                    record.push(format!("retained{}", len));
                })
            };

            src.retain(|_, _| true);
            assert_eq!(src.state.state, state); // state is unchanged
            assert_eq!(record1.lock().unwrap().len(), 0); // retain method does not send any message
            assert_eq!(record2.lock().unwrap().len(), 0); // retain method does not send any message
            assert!(src.data.is_empty());

            src.retain(|_, _| false);
            assert_eq!(src.state.state, state); // state is unchanged
            assert_eq!(record1.lock().unwrap().len(), 0); // retain method does not send any message
            assert_eq!(record2.lock().unwrap().len(), 0); // retain method does not send any message
            assert!(src.data.is_empty());
        }
    }

    mod args2 {
        use super::*;

        #[fixture]
        fn data() -> HashMap<&'static str, HashMap<&'static str, i32>> {
            hashmap! {
                "a" => hashmap!{
                    "x" => 1,
                    "y" => 2
                },
                "b" => hashmap!{
                    "x" => 3,
                    "y" => 4
                },
                "c" => hashmap!{
                    "x" => 5,
                    "y" => 6
                },
            }
        }

        #[rstest]
        fn test_default() {
            let src = OnMemorySrc2Args::<&'static str, &'static str, i32>::default();
            assert!(src.data.is_empty());
            assert!(src.state.obs.is_empty());
        }

        #[rstest]
        fn test_from(data: HashMap<&'static str, HashMap<&'static str, i32>>) {
            let src = OnMemorySrc2Args::from(data.clone());
            assert_eq!(src.data, data);
            assert!(src.state.obs.is_empty());
        }

        #[rstest]
        fn test_from_iter(data: HashMap<&'static str, HashMap<&'static str, i32>>) {
            let src = OnMemorySrc2Args::from_iter(data.clone().into_iter());
            assert_eq!(src.data, data);
            assert!(src.state.obs.is_empty());
        }

        #[rstest]
        fn test_new() {
            let src = OnMemorySrc2Args::<&'static str, &'static str, i32>::new();
            assert!(src.data.is_empty());
            assert!(src.state.obs.is_empty());
        }

        #[rstest]
        fn test_clone(data: HashMap<&'static str, HashMap<&'static str, i32>>) {
            let mut src = OnMemorySrc2Args::from(data);
            let state = src.state.state;
            let record = Arc::new(Mutex::new(Vec::new()));
            let _ = {
                let record = record.clone();
                src.on_change(move |id| {
                    record.lock().unwrap().push(*id);
                })
            };
            assert_eq!(src.state.obs.len(), 1);

            // data is cloned but observers are not cloned
            let cloned = src.clone();
            assert_eq!(src.state.state, state); // state is unchanged
            assert_eq!(record.lock().unwrap().len(), 0); // clone does not send any message
            assert_eq!(src.data, cloned.data);
            assert!(cloned.state.obs.is_empty());
            assert_ne!(src.state.state, cloned.state.state); // this is probabilistic

            // empty
            let mut src = OnMemorySrc2Args::<&'static str, &'static str, i32>::new();
            let state = src.state.state;
            let _ = {
                let record = record.clone();
                src.on_change(move |id| {
                    record.lock().unwrap().push(*id);
                })
            };
            assert_eq!(src.state.obs.len(), 1);

            let cloned = src.clone();
            assert_eq!(src.state.state, state); // state is unchanged
            assert!(cloned.data.is_empty());
            assert_eq!(record.lock().unwrap().len(), 0); // clone does not send any message
            assert!(cloned.state.obs.is_empty());
            assert_ne!(src.state.state, cloned.state.state); // this is probabilistic
        }

        #[rstest]
        fn test_req(data: HashMap<&'static str, HashMap<&'static str, i32>>) {
            // non-empty
            let mut src = OnMemorySrc2Args::from(data.clone());
            let state = src.state.state;
            let record = Arc::new(Mutex::new(Vec::new()));
            let _ = {
                let record = record.clone();
                src.on_change(move |id| {
                    record.lock().unwrap().push(*id);
                })
            };
            assert_eq!(src.state.obs.len(), 1);

            // ok
            assert_eq!(src.req(&"a", &"x").unwrap(), 1);
            assert_eq!(src.state.state, state); // state is unchanged
            assert_eq!(record.lock().unwrap().len(), 0); // req method does not send any message
            assert_eq!(src.req(&"b", &"y").unwrap(), 4);
            assert_eq!(src.state.state, state); // state is unchanged
            assert_eq!(record.lock().unwrap().len(), 0); // req method does not send any message
            assert_eq!(src.req(&"c", &"x").unwrap(), 5);
            assert_eq!(src.state.state, state); // state is unchanged
            assert_eq!(record.lock().unwrap().len(), 0); // req method does not send any message

            // error
            assert!(src.req(&"d", &"x").is_err());
            assert_eq!(src.state.state, state); // state is unchanged
            assert_eq!(record.lock().unwrap().len(), 0); // req method does not send any message

            // empty
            let mut src = OnMemorySrc2Args::<&'static str, &'static str, i32>::new();
            let state = src.state.state;
            let record = Arc::new(Mutex::new(Vec::new()));
            let _ = {
                let record = record.clone();
                src.on_change(move |id| {
                    record.lock().unwrap().push(*id);
                })
            };

            assert!(src.req(&"a", &"x").is_err());
            assert_eq!(src.state.state, state); // state is unchanged
            assert_eq!(record.lock().unwrap().len(), 0); // req method does not send any message
            assert!(src.req(&"b", &"y").is_err());
            assert_eq!(src.state.state, state); // state is unchanged
            assert_eq!(record.lock().unwrap().len(), 0); // req method does not send any message
        }

        #[rstest]
        fn test_take_snapshot(data: HashMap<&'static str, HashMap<&'static str, i32>>) {
            // non-empty
            let mut src = OnMemorySrc2Args::from(data.clone());
            let state = src.state.state;
            let record = Arc::new(Mutex::new(Vec::new()));
            let _ = {
                let record = record.clone();
                src.on_change(move |id| {
                    record.lock().unwrap().push(*id);
                })
            };
            assert_eq!(src.state.obs.len(), 1);

            // ok
            let snapshot = src.take_snapshot(Vec::<(_, Vec<_>)>::default()).unwrap();
            assert_eq!(src.state.state, state); // state is unchanged
            assert_ne!(snapshot.state.state, state); // this is probabilistic
            assert_eq!(record.lock().unwrap().len(), 0); // take_snapshot method does not send any message
            assert!(snapshot.data.is_empty());
            assert_eq!(snapshot.state.obs.len(), 0);

            let snapshot = src.take_snapshot(&hashmap! {"a" => ["y"]}).unwrap();
            assert_eq!(src.state.state, state); // state is unchanged
            assert_ne!(snapshot.state.state, state); // this is probabilistic
            assert_eq!(record.lock().unwrap().len(), 0); // take_snapshot method does not send any message
            assert_eq!(snapshot.data, hashmap! {"a" => hashmap!{"y" => 2}});
            assert_eq!(snapshot.state.obs.len(), 0);

            let snapshot = src
                .take_snapshot(&hashmap! {"a" => vec!["x"], "b" => vec!["x", "y"]})
                .unwrap();
            assert_eq!(src.state.state, state); // state is unchanged
            assert_ne!(snapshot.state.state, state); // this is probabilistic
            assert_eq!(record.lock().unwrap().len(), 0); // take_snapshot method does not send any message
            assert_eq!(
                snapshot.data,
                hashmap! {
                    "a" => hashmap!{"x" => 1},
                    "b" => hashmap!{"x" => 3, "y" => 4}
                }
            );
            assert_eq!(snapshot.state.obs.len(), 0);

            // error
            assert!(src.take_snapshot(&hashmap! {"d" => vec!["x"]}).is_err());
            assert_eq!(src.state.state, state); // state is unchanged
            assert_eq!(record.lock().unwrap().len(), 0); // take_snapshot method does not send any message

            // empty
            let mut src = OnMemorySrc2Args::<&'static str, &'static str, i32>::new();
            let state = src.state.state;
            let record = Arc::new(Mutex::new(Vec::new()));
            let _ = {
                let record = record.clone();
                src.on_change(move |id| {
                    record.lock().unwrap().push(*id);
                })
            };
            assert_eq!(src.state.obs.len(), 1);

            let snapshot = src.take_snapshot(Vec::<(_, Vec<_>)>::default()).unwrap();
            assert_eq!(src.state.state, state); // state is unchanged
            assert_ne!(snapshot.state.state, state); // this is probabilistic
            assert_eq!(record.lock().unwrap().len(), 0); // take_snapshot method does not send any message
            assert!(snapshot.data.is_empty());
            assert_eq!(snapshot.state.obs.len(), 0);

            assert!(src.take_snapshot(&hashmap! {"a" => ["y"]}).is_err());
            assert_eq!(src.state.state, state); // state is unchanged
            assert_eq!(record.lock().unwrap().len(), 0); // take_snapshot method does not send any message
        }

        #[rstest]
        fn test_inner(data: HashMap<&'static str, HashMap<&'static str, i32>>) {
            // non-empty
            let mut src = OnMemorySrc2Args::from(data.clone());
            let state = src.state.state;
            let record = Arc::new(Mutex::new(Vec::new()));
            let _ = {
                let record = record.clone();
                src.on_change(move |id| {
                    record.lock().unwrap().push(*id);
                })
            };
            assert_eq!(src.state.obs.len(), 1);
            assert_eq!(src.inner(), &data);
            assert_eq!(src.state.state, state); // state is unchanged
            assert_eq!(record.lock().unwrap().len(), 0); // inner method does not send any message

            // empty
            let mut src = OnMemorySrc2Args::<&'static str, &'static str, i32>::new();
            let state = src.state.state;
            let _ = {
                let record = record.clone();
                src.on_change(move |id| {
                    record.lock().unwrap().push(*id);
                })
            };
            assert_eq!(src.state.obs.len(), 1);
            assert!(src.inner().is_empty());
            assert_eq!(src.state.state, state); // state is unchanged
            assert_eq!(record.lock().unwrap().len(), 0); // inner method does not send any message
        }

        #[rstest]
        fn test_into_inner(data: HashMap<&'static str, HashMap<&'static str, i32>>) {
            // non-empty
            let mut src = OnMemorySrc2Args::from(data.clone());
            let record = Arc::new(Mutex::new(Vec::new()));
            let _ = {
                let record = record.clone();
                src.on_change(move |id| {
                    record.lock().unwrap().push(*id);
                })
            };
            assert_eq!(src.state.obs.len(), 1);
            assert_eq!(src.into_inner(), data);
            assert_eq!(record.lock().unwrap().len(), 0); // into_inner method does not send any message

            // empty
            let mut src = OnMemorySrc2Args::<&'static str, &'static str, i32>::new();
            let _ = {
                let record = record.clone();
                src.on_change(move |id| {
                    record.lock().unwrap().push(*id);
                })
            };
            assert_eq!(src.state.obs.len(), 1);

            assert!(src.into_inner().is_empty());
            assert_eq!(record.lock().unwrap().len(), 0); // into_inner method does not send any message
        }

        #[rstest]
        fn test_len(data: HashMap<&'static str, HashMap<&'static str, i32>>) {
            // non-empty
            let mut src = OnMemorySrc2Args::from(data.clone());
            let record = Arc::new(Mutex::new(Vec::new()));
            let _ = {
                let record = record.clone();
                src.on_change(move |id| {
                    record.lock().unwrap().push(*id);
                })
            };
            let state = src.state.state;
            assert_eq!(src.len(), 6);
            assert_eq!(src.state.state, state); // state is unchanged
            assert_eq!(record.lock().unwrap().len(), 0); // len method does not send any message

            // empty
            let mut src = OnMemorySrc2Args::<&'static str, &'static str, i32>::new();
            let record = Arc::new(Mutex::new(Vec::new()));
            let _ = {
                let record = record.clone();
                src.on_change(move |id| {
                    record.lock().unwrap().push(*id);
                })
            };
            let state = src.state.state;
            assert_eq!(src.len(), 0);
            assert_eq!(src.state.state, state); // state is unchanged
            assert_eq!(record.lock().unwrap().len(), 0); // len method does not send any message
        }

        #[rstest]
        fn test_is_empty(data: HashMap<&'static str, HashMap<&'static str, i32>>) {
            // non-empty
            let mut src = OnMemorySrc2Args::from(data.clone());
            let record = Arc::new(Mutex::new(Vec::new()));
            let _ = {
                let record = record.clone();
                src.on_change(move |id| {
                    record.lock().unwrap().push(*id);
                })
            };
            let state = src.state.state;
            assert!(!src.is_empty());
            assert_eq!(src.state.state, state); // state is unchanged
            assert_eq!(record.lock().unwrap().len(), 0); // is_empty method does not send any message

            // empty
            let mut src = OnMemorySrc2Args::<&'static str, &'static str, i32>::new();
            let record = Arc::new(Mutex::new(Vec::new()));
            let _ = {
                let record = record.clone();
                src.on_change(move |id| {
                    record.lock().unwrap().push(*id);
                })
            };
            let state = src.state.state;
            assert!(src.is_empty());
            assert_eq!(src.state.state, state); // state is unchanged
            assert_eq!(record.lock().unwrap().len(), 0); // is_empty method does not send any message
        }

        #[rstest]
        fn test_contains_key(data: HashMap<&'static str, HashMap<&'static str, i32>>) {
            // non-empty
            let mut src = OnMemorySrc2Args::from(data.clone());
            let record = Arc::new(Mutex::new(Vec::new()));
            let _ = {
                let record = record.clone();
                src.on_change(move |id| {
                    record.lock().unwrap().push(*id);
                })
            };

            let state = src.state.state;
            assert!(src.contains_key(&"a", &"x"));
            assert_eq!(src.state.state, state); // state is unchanged
            assert_eq!(record.lock().unwrap().len(), 0); // contains_key method does not send any message

            assert!(src.contains_key(&"b", &"y"));
            assert_eq!(src.state.state, state); // state is unchanged
            assert_eq!(record.lock().unwrap().len(), 0); // contains_key method does not send any message
            assert!(!src.contains_key(&"c", &"z"));
            assert_eq!(src.state.state, state); // state is unchanged
            assert_eq!(record.lock().unwrap().len(), 0); // contains_key method does not send any message

            // empty
            let mut src = OnMemorySrc2Args::<&'static str, &'static str, i32>::new();
            let record = Arc::new(Mutex::new(Vec::new()));
            let _ = {
                let record = record.clone();
                src.on_change(move |id| {
                    record.lock().unwrap().push(*id);
                })
            };
            let state = src.state.state;
            assert!(!src.contains_key(&"a", &"x"));
            assert_eq!(src.state.state, state); // state is unchanged
            assert_eq!(record.lock().unwrap().len(), 0); // contains_key method does not send any message
        }

        #[rstest]
        fn test_get(data: HashMap<&'static str, HashMap<&'static str, i32>>) {
            // non-empty
            let mut src = OnMemorySrc2Args::from(data.clone());
            let state = src.state.state;
            let record = Arc::new(Mutex::new(Vec::new()));
            let _ = {
                let record = record.clone();
                src.on_change(move |id| {
                    record.lock().unwrap().push(*id);
                })
            };
            assert_eq!(src.state.obs.len(), 1);

            assert_eq!(src.get(&"a", &"x"), Some(&1));
            assert_eq!(src.state.state, state); // state is unchanged
            assert_eq!(record.lock().unwrap().len(), 0); // get method does not send any message
            assert_eq!(src.get(&"b", &"y"), Some(&4));
            assert_eq!(src.state.state, state); // state is unchanged
            assert_eq!(record.lock().unwrap().len(), 0); // get method does not send any message
            assert_eq!(src.get(&"c", &"z"), None);
            assert_eq!(src.state.state, state); // state is unchanged
            assert_eq!(record.lock().unwrap().len(), 0); // get method does not send any message

            // empty
            let mut src = OnMemorySrc2Args::<&'static str, &'static str, i32>::new();
            let state = src.state.state;
            let _ = {
                let record = record.clone();
                src.on_change(move |id| {
                    record.lock().unwrap().push(*id);
                })
            };
            assert_eq!(src.state.obs.len(), 1);

            assert_eq!(src.get(&"a", &"x"), None);
            assert_eq!(src.state.state, state); // state is unchanged
            assert_eq!(record.lock().unwrap().len(), 0); // get method does not send any message
            assert_eq!(src.get(&"b", &"y"), None);
            assert_eq!(src.state.state, state); // state is unchanged
            assert_eq!(record.lock().unwrap().len(), 0); // get method does not send any message
        }

        #[rstest]
        fn test_insert(data: HashMap<&'static str, HashMap<&'static str, i32>>) {
            let record1 = Arc::new(Mutex::new(Vec::new()));
            let record2 = Arc::new(Mutex::new(Vec::new()));

            // non-empty
            let mut src = OnMemorySrc2Args::from(data.clone());
            let mut state = src.state.state;
            let _1 = {
                let record1 = record1.clone();
                src.on_change(move |id| {
                    record1.lock().unwrap().push(*id);
                })
            };
            let _2 = {
                let record2 = record2.clone();
                src.on_change(move |_| {
                    let mut record = record2.lock().unwrap();
                    let len = record.len();
                    record.push(format!("inserted{}", len));
                })
            };
            assert_eq!(src.state.obs.len(), 2);

            assert_eq!(src.insert("a", "x", 10), Some(1));
            assert_ne!(src.state.state, state); // state is changed
            assert_eq!(record1.lock().unwrap().len(), 1); // insert method sends a message
            assert_eq!(record1.lock().unwrap()[0], src.state.state);
            assert_eq!(record2.lock().unwrap().len(), 1); // insert method sends a message
            assert_eq!(record2.lock().unwrap()[0], "inserted0");
            state = src.state.state;

            assert_eq!(src.insert("b", "y", 20), Some(4));
            assert_ne!(src.state.state, state); // state is changed
            assert_eq!(record1.lock().unwrap().len(), 2); // insert method sends a message
            assert_eq!(record1.lock().unwrap()[1], src.state.state);
            assert_eq!(record2.lock().unwrap().len(), 2); // insert method sends a message
            assert_eq!(record2.lock().unwrap()[1], "inserted1");

            assert_eq!(src.insert("c", "z", 30), None);
            assert_ne!(src.state.state, state); // state is changed
            assert_eq!(record1.lock().unwrap().len(), 3); // insert method sends a message
            assert_eq!(record1.lock().unwrap()[2], src.state.state);
            assert_eq!(record2.lock().unwrap().len(), 3); // insert method sends a message

            assert_eq!(
                src.data,
                hashmap! {
                    "a" => hashmap!{"x" => 10, "y" => 2},
                    "b" => hashmap!{"x" => 3, "y" => 20},
                    "c" => hashmap!{"x" => 5, "y" => 6, "z" => 30}
                }
            );

            // empty
            let mut src = OnMemorySrc2Args::<&'static str, &'static str, i32>::new();
            let state = src.state.state;
            let _1 = {
                let record1 = record1.clone();
                record1.lock().unwrap().clear();
                src.on_change(move |id| {
                    record1.lock().unwrap().push(*id);
                })
            };
            let _2 = {
                let record2 = record2.clone();
                record2.lock().unwrap().clear();
                src.on_change(move |_| {
                    let mut record = record2.lock().unwrap();
                    let len = record.len();
                    record.push(format!("inserted{}", len));
                })
            };
            assert_eq!(src.state.obs.len(), 2);

            assert_eq!(src.insert("a", "x", 10), None);
            assert_ne!(src.state.state, state);
            assert_eq!(record1.lock().unwrap().len(), 1); // insert method sends a message
            assert_eq!(record1.lock().unwrap()[0], src.state.state);
            assert_eq!(record2.lock().unwrap().len(), 1); // insert method sends a message
            assert_eq!(record2.lock().unwrap()[0], "inserted0");
            assert_eq!(src.data, hashmap! {"a" => hashmap!{"x" => 10}});
        }

        #[rstest]
        fn test_remove(data: HashMap<&'static str, HashMap<&'static str, i32>>) {
            let record1 = Arc::new(Mutex::new(Vec::new()));
            let record2 = Arc::new(Mutex::new(Vec::new()));

            // non-empty
            let mut src = OnMemorySrc2Args::from(data.clone());
            let mut state = src.state.state;
            let _1 = {
                let record1 = record1.clone();
                src.on_change(move |id| {
                    record1.lock().unwrap().push(*id);
                })
            };
            let _2 = {
                let record2 = record2.clone();
                src.on_change(move |_| {
                    let mut record = record2.lock().unwrap();
                    let len = record.len();
                    record.push(format!("removed{}", len));
                })
            };
            assert_eq!(src.state.obs.len(), 2);

            assert_eq!(src.remove(&"a", &"x"), Some(1));
            assert_ne!(src.state.state, state); // state is changed
            assert_eq!(record1.lock().unwrap().len(), 1); // remove method sends a message
            assert_eq!(record1.lock().unwrap()[0], src.state.state);
            assert_eq!(record2.lock().unwrap().len(), 1); // remove method sends a message
            assert_eq!(record2.lock().unwrap()[0], "removed0");
            state = src.state.state;

            assert_eq!(src.remove(&"c", &"y"), Some(6));
            assert_ne!(src.state.state, state); // state is changed
            assert_eq!(record1.lock().unwrap().len(), 2); // remove method sends a message
            assert_eq!(record1.lock().unwrap()[1], src.state.state);
            assert_eq!(record2.lock().unwrap().len(), 2); // remove method sends a message
            assert_eq!(record2.lock().unwrap()[1], "removed1");
            state = src.state.state;

            assert_eq!(src.remove(&"d", &"z"), None);
            assert_eq!(src.state.state, state); // state is unchanged
            assert_eq!(record1.lock().unwrap().len(), 2); // remove method does not send any message
            assert_eq!(record2.lock().unwrap().len(), 2); // remove method does not send any message
            assert_eq!(
                src.data,
                hashmap! {
                    "a" => hashmap!{"y" => 2},
                    "b" => hashmap!{"x" => 3, "y" => 4},
                    "c" => hashmap!{"x" => 5}
                }
            );

            // empty
            let mut src = OnMemorySrc2Args::<&'static str, &'static str, i32>::new();
            let state = src.state.state;
            let _1 = {
                let record1 = record1.clone();
                record1.lock().unwrap().clear();
                src.on_change(move |id| {
                    record1.lock().unwrap().push(*id);
                })
            };
            let _2 = {
                let record2 = record2.clone();
                record2.lock().unwrap().clear();
                src.on_change(move |_| {
                    let mut record = record2.lock().unwrap();
                    let len = record.len();
                    record.push(format!("removed{}", len));
                })
            };
            assert_eq!(src.state.obs.len(), 2);

            assert_eq!(src.remove(&"a", &"x"), None);
            assert_eq!(src.state.state, state); // state is unchanged
            assert_eq!(record1.lock().unwrap().len(), 0); // remove method does not send any message
            assert_eq!(record2.lock().unwrap().len(), 0); // remove method does not send any message
            assert!(src.data.is_empty());
        }

        #[rstest]
        fn test_remove_entry(data: HashMap<&'static str, HashMap<&'static str, i32>>) {
            let record1 = Arc::new(Mutex::new(Vec::new()));
            let record2 = Arc::new(Mutex::new(Vec::new()));

            // non-empty
            let mut src = OnMemorySrc2Args::from(data.clone());
            let mut state = src.state.state;
            let _1 = {
                let record1 = record1.clone();
                src.on_change(move |id| {
                    record1.lock().unwrap().push(*id);
                })
            };
            let _2 = {
                let record2 = record2.clone();
                src.on_change(move |_| {
                    let mut record = record2.lock().unwrap();
                    let len = record.len();
                    record.push(format!("removed{}", len));
                })
            };
            assert_eq!(src.state.obs.len(), 2);

            assert_eq!(src.remove_entry(&"a", &"x"), Some(("a", "x", 1)));
            assert_ne!(src.state.state, state); // state is changed
            assert_eq!(record1.lock().unwrap().len(), 1); // remove_entry method sends a message
            assert_eq!(record1.lock().unwrap()[0], src.state.state);
            assert_eq!(record2.lock().unwrap().len(), 1); // remove_entry method sends a message
            assert_eq!(record2.lock().unwrap()[0], "removed0");
            state = src.state.state;

            assert_eq!(src.remove_entry(&"c", &"y"), Some(("c", "y", 6)));
            assert_ne!(src.state.state, state); // state is changed
            assert_eq!(record1.lock().unwrap().len(), 2); // remove_entry method sends a message
            assert_eq!(record1.lock().unwrap()[1], src.state.state);
            assert_eq!(record2.lock().unwrap().len(), 2); // remove_entry method sends a message
            assert_eq!(record2.lock().unwrap()[1], "removed1");
            state = src.state.state;

            assert_eq!(src.remove_entry(&"d", &"z"), None);
            assert_eq!(src.state.state, state); // state is unchanged
            assert_eq!(record1.lock().unwrap().len(), 2); // remove_entry method does not send any message
            assert_eq!(record2.lock().unwrap().len(), 2); // remove_entry method does not send any message
            assert_eq!(
                src.data,
                hashmap! {
                    "a" => hashmap!{"y" => 2},
                    "b" => hashmap!{"x" => 3, "y" => 4},
                    "c" => hashmap!{"x" => 5}
                }
            );

            // empty
            let mut src = OnMemorySrc2Args::<&'static str, &'static str, i32>::new();
            let _state = src.state.state;
            let _1 = {
                let record1 = record1.clone();
                record1.lock().unwrap().clear();
                src.on_change(move |id| {
                    record1.lock().unwrap().push(*id);
                })
            };
            let _2 = {
                let record2 = record2.clone();
                record2.lock().unwrap().clear();
                src.on_change(move |_| {
                    let mut record = record2.lock().unwrap();
                    let len = record.len();
                    record.push(format!("removed{}", len));
                })
            };
            assert_eq!(src.state.obs.len(), 2);
        }

        #[rstest]
        fn test_clear(data: HashMap<&'static str, HashMap<&'static str, i32>>) {
            let record1 = Arc::new(Mutex::new(Vec::new()));
            let record2 = Arc::new(Mutex::new(Vec::new()));

            // non-empty
            let mut src = OnMemorySrc2Args::from(data.clone());
            let state = src.state.state;
            let _1 = {
                let record1 = record1.clone();
                src.on_change(move |id| {
                    record1.lock().unwrap().push(*id);
                })
            };
            let _2 = {
                let record2 = record2.clone();
                src.on_change(move |_| {
                    let mut record = record2.lock().unwrap();
                    let len = record.len();
                    record.push(format!("cleared{}", len));
                })
            };
            assert_eq!(src.state.obs.len(), 2);

            src.clear();
            assert_ne!(src.state.state, state); // state is changed
            assert_eq!(record1.lock().unwrap().len(), 1); // clear method sends a message
            assert_eq!(record1.lock().unwrap()[0], src.state.state);
            assert_eq!(record2.lock().unwrap().len(), 1); // clear method sends a message
            assert_eq!(record2.lock().unwrap()[0], "cleared0");
            assert!(src.data.is_empty());
            let state = src.state.state;

            src.clear();
            assert_eq!(src.state.state, state); // state is unchanged
            assert_eq!(record1.lock().unwrap().len(), 1); // clear method does not send any message
            assert_eq!(record2.lock().unwrap().len(), 1); // clear method does not send any message
            assert!(src.data.is_empty());

            // empty
            let mut src = OnMemorySrc2Args::<&'static str, &'static str, i32>::new();
            let state = src.state.state;
            let _1 = {
                let record1 = record1.clone();
                record1.lock().unwrap().clear();
                src.on_change(move |id| {
                    record1.lock().unwrap().push(*id);
                })
            };
            let _2 = {
                let record2 = record2.clone();
                record2.lock().unwrap().clear();
                src.on_change(move |_| {
                    let mut record = record2.lock().unwrap();
                    let len = record.len();
                    record.push(format!("cleared{}", len));
                })
            };
            assert_eq!(src.state.obs.len(), 2);

            src.clear();
            assert_eq!(src.state.state, state); // state is unchanged
            assert_eq!(record1.lock().unwrap().len(), 0); // clear method does not send any message
            assert_eq!(record2.lock().unwrap().len(), 0); // clear method does not send any message
            assert!(src.data.is_empty());
        }

        #[rstest]
        fn test_retain(data: HashMap<&'static str, HashMap<&'static str, i32>>) {
            let record1 = Arc::new(Mutex::new(Vec::new()));
            let record2 = Arc::new(Mutex::new(Vec::new()));

            // non-empty
            let mut src = OnMemorySrc2Args::from(data.clone());
            let state = src.state.state;
            let _1 = {
                let record1 = record1.clone();
                src.on_change(move |id| {
                    record1.lock().unwrap().push(*id);
                })
            };
            let _2 = {
                let record2 = record2.clone();
                src.on_change(move |_| {
                    let mut record = record2.lock().unwrap();
                    let len = record.len();
                    record.push(format!("retained{}", len));
                })
            };
            assert_eq!(src.state.obs.len(), 2);

            src.retain(|_, _, _| true);
            assert_eq!(src.state.state, state); // state is unchanged
            assert_eq!(record1.lock().unwrap().len(), 0); // retain method does not send any message
            assert_eq!(record2.lock().unwrap().len(), 0); // retain method does not send any message
            assert_eq!(src.data, data);

            src.retain(|c, _, _| c != &"b");
            assert_ne!(src.state.state, state); // state is changed
            assert_eq!(record1.lock().unwrap().len(), 1); // retain method sends a message
            assert_eq!(record1.lock().unwrap()[0], src.state.state);
            assert_eq!(record2.lock().unwrap().len(), 1); // retain method sends a message
            assert_eq!(record2.lock().unwrap()[0], "retained0");
            assert_eq!(
                src.data,
                hashmap! {"a" => hashmap!{"x" => 1, "y" => 2}, "c" => hashmap!{"x" => 5, "y" => 6}}
            );
            let state = src.state.state;

            src.retain(|_, _, v| *v % 2 == 0);
            assert_ne!(src.state.state, state); // state is changed
            assert_eq!(record1.lock().unwrap().len(), 2); // retain method sends a message
            assert_eq!(record1.lock().unwrap()[1], src.state.state);
            assert_eq!(record2.lock().unwrap().len(), 2); // retain method sends a message
            assert_eq!(record2.lock().unwrap()[1], "retained1");
            assert_eq!(
                src.data,
                hashmap! {"a" => hashmap!{"y" => 2}, "c" => hashmap!{"y" => 6}}
            );
            let state = src.state.state;

            src.retain(|c, _, _| c != &"c");
            assert_ne!(src.state.state, state); // state is changed
            assert_eq!(record1.lock().unwrap().len(), 3); // retain method sends a message
            assert_eq!(record1.lock().unwrap()[2], src.state.state);
            assert_eq!(record2.lock().unwrap().len(), 3); // retain method sends a message
            assert_eq!(record2.lock().unwrap()[2], "retained2");
            assert_eq!(src.data, hashmap! {"a" => hashmap!{"y" => 2}});

            // empty
            let mut src = OnMemorySrc2Args::<&'static str, &'static str, i32>::new();
            let state = src.state.state;
            let _1 = {
                let record1 = record1.clone();
                record1.lock().unwrap().clear();
                src.on_change(move |id| {
                    record1.lock().unwrap().push(*id);
                })
            };
            let _2 = {
                let record2 = record2.clone();
                record2.lock().unwrap().clear();
                src.on_change(move |_| {
                    let mut record = record2.lock().unwrap();
                    let len = record.len();
                    record.push(format!("retained{}", len));
                })
            };
            assert_eq!(src.state.obs.len(), 2);

            src.retain(|_, _, _| true);
            assert_eq!(src.state.state, state); // state is unchanged
            assert_eq!(record1.lock().unwrap().len(), 0); // retain method does not send any message
            assert_eq!(record2.lock().unwrap().len(), 0); // retain method does not send any message
            assert!(src.data.is_empty());
        }
    }

    mod args3 {
        use super::*;

        type DataType = HashMap<&'static str, HashMap<&'static str, HashMap<&'static str, i32>>>;
        type SrcType = OnMemorySrc3Args<&'static str, &'static str, &'static str, i32>;

        #[fixture]
        fn data() -> DataType {
            hashmap! {
                "a" => hashmap!{
                    "x" => hashmap!{
                        "i" => 1,
                        "j" => 2
                    },
                    "y" => hashmap!{
                        "i" => 3,
                        "j" => 4
                    }
                },
                "b" => hashmap!{
                    "x" => hashmap!{
                        "i" => 5,
                        "j" => 6
                    },
                    "y" => hashmap!{
                        "i" => 7,
                        "j" => 8
                    }
                },
                "c" => hashmap!{
                    "x" => hashmap!{
                        "i" => 9,
                        "j" => 10
                    },
                    "y" => hashmap!{
                        "i" => 11,
                        "j" => 12
                    }
                },
            }
        }

        #[test]
        fn test_default() {
            let src = SrcType::default();
            assert!(src.data.is_empty());
            assert!(src.state.obs.is_empty());

            let src2 = SrcType::default();
            assert_ne!(src.state.state, src2.state.state); // this is probabilistic
        }

        #[rstest]
        fn test_from(data: DataType) {
            // non-empty
            let src = SrcType::from(data.clone());
            assert_eq!(src.data, data);
            assert!(src.state.obs.is_empty());

            let src2 = SrcType::from(data);
            assert_ne!(src.state.state, src2.state.state); // this is probabilistic

            // empty
            let src = SrcType::from(DataType::default());
            assert!(src.data.is_empty());
            assert!(src.state.obs.is_empty());

            let src2 = SrcType::from(DataType::default());
            assert_ne!(src.state.state, src2.state.state); // this is probabilistic
        }

        #[rstest]
        fn test_from_iter(data: DataType) {
            // non-empty
            let src = SrcType::from_iter(data.clone());
            assert_eq!(src.data, data);
            assert!(src.state.obs.is_empty());

            let src2 = SrcType::from_iter(data);
            assert_ne!(src.state.state, src2.state.state); // this is probabilistic

            // empty
            let src = SrcType::from_iter(DataType::default());
            assert!(src.data.is_empty());
            assert!(src.state.obs.is_empty());

            let src2 = SrcType::from_iter(DataType::default());
            assert_ne!(src.state.state, src2.state.state); // this is probabilistic
        }

        #[rstest]
        fn test_new() {
            let src = SrcType::new();
            assert!(src.data.is_empty());
            assert!(src.state.obs.is_empty());

            let src2 = SrcType::new();
            assert_ne!(src.state.state, src2.state.state); // this is probabilistic
        }

        #[rstest]
        fn test_clone(data: DataType) {
            let mut src = SrcType::from(data.clone());
            let state = src.state.state;
            let record = Arc::new(Mutex::new(Vec::new()));
            let _ = {
                let record = record.clone();
                src.on_change(move |id| {
                    record.lock().unwrap().push(*id);
                })
            };
            assert_eq!(src.state.obs.len(), 1);

            // data is cloned but state is not cloned
            let cloned = src.clone();
            assert_eq!(src.state.state, state); // state is unchanged
            assert_eq!(record.lock().unwrap().len(), 0); // clone does not send any message
            assert_eq!(src.data, cloned.data);
            assert!(cloned.state.obs.is_empty());
            assert_ne!(src.state.state, cloned.state.state); // this is probabilistic

            // empty
            let mut src = SrcType::new();
            let state = src.state.state;
            let record = Arc::new(Mutex::new(Vec::new()));
            let _ = {
                let record = record.clone();
                src.on_change(move |id| {
                    record.lock().unwrap().push(*id);
                })
            };

            let cloned = src.clone();
            assert_eq!(src.state.state, state); // state is unchanged
            assert_eq!(record.lock().unwrap().len(), 0); // clone does not send any message
            assert!(src.data.is_empty());
            assert!(cloned.data.is_empty());
            assert!(cloned.state.obs.is_empty());
            assert_ne!(src.state.state, cloned.state.state); // this is probabilistic
        }

        #[rstest]
        fn test_req(data: DataType) {
            // non-empty
            let mut src = SrcType::from(data.clone());
            let state = src.state.state;
            let record = Arc::new(Mutex::new(Vec::new()));
            let _ = {
                let record = record.clone();
                src.on_change(move |id| {
                    record.lock().unwrap().push(*id);
                })
            };

            // ok
            assert_eq!(src.req(&"a", &"x", &"i").unwrap(), 1);
            assert_eq!(src.state.state, state); // state is unchanged
            assert_eq!(record.lock().unwrap().len(), 0); // req method does not send any message
            assert_eq!(src.req(&"b", &"y", &"j").unwrap(), 8);
            assert_eq!(src.state.state, state); // state is unchanged
            assert_eq!(record.lock().unwrap().len(), 0); // req method does not send any message

            // error
            assert!(src.req(&"c", &"z", &"k").is_err());
            assert_eq!(src.state.state, state); // state is unchanged
            assert_eq!(record.lock().unwrap().len(), 0); // req method does not send any message

            // empty
            let mut src = SrcType::new();
            let state = src.state.state;
            let record = Arc::new(Mutex::new(Vec::new()));
            let _ = {
                let record = record.clone();
                src.on_change(move |id| {
                    record.lock().unwrap().push(*id);
                })
            };

            // error
            assert!(src.req(&"a", &"x", &"i").is_err());
            assert_eq!(src.state.state, state); // state is unchanged
            assert_eq!(record.lock().unwrap().len(), 0); // req method does not send any message
        }

        #[rstest]
        fn test_take_snapshot(data: DataType) {
            // non-empty
            let mut src = SrcType::from(data.clone());
            let state = src.state.state;
            let record = Arc::new(Mutex::new(Vec::new()));
            let _ = {
                let record = record.clone();
                src.on_change(move |id| {
                    record.lock().unwrap().push(*id);
                })
            };
            assert_eq!(src.state.obs.len(), 1);

            let snapshot = src
                .take_snapshot(&HashMap::<_, HashMap<_, Vec<_>>>::default())
                .unwrap();
            assert_eq!(src.state.state, state); // state is unchanged
            assert_ne!(snapshot.state.state, state); // this is probabilistic
            assert_eq!(record.lock().unwrap().len(), 0); // take_snapshot method does not send any message
            assert!(snapshot.data.is_empty());
            assert_eq!(snapshot.state.obs.len(), 0);

            let snapshot = src
                .take_snapshot(&hashmap! {"a" => hashmap!{"x" => vec!["i"]}})
                .unwrap();
            assert_eq!(src.state.state, state); // state is unchanged
            assert_ne!(snapshot.state.state, state); // this is probabilistic
            assert_eq!(record.lock().unwrap().len(), 0); // take_snapshot method does not send any message
            assert_eq!(
                snapshot.data,
                hashmap! {"a" => hashmap!{"x" => hashmap!{"i" => 1}}}
            );
            assert_eq!(snapshot.state.obs.len(), 0);

            // error
            assert!(src
                .take_snapshot(&hashmap! {"a" => hashmap!{"x" => vec!["k"]}})
                .is_err());
            assert_eq!(src.state.state, state); // state is unchanged
            assert_eq!(record.lock().unwrap().len(), 0); // take_snapshot method does not send any message

            // empty
            let mut src = SrcType::new();
            let state = src.state.state;
            let record = Arc::new(Mutex::new(Vec::new()));
            let _ = {
                let record = record.clone();
                src.on_change(move |id| {
                    record.lock().unwrap().push(*id);
                })
            };
            assert_eq!(src.state.obs.len(), 1);

            // ok
            let snapshot = src
                .take_snapshot(&HashMap::<_, HashMap<_, Vec<_>>>::default())
                .unwrap();
            assert_eq!(src.state.state, state); // state is unchanged
            assert_ne!(snapshot.state.state, state); // this is probabilistic
            assert_eq!(record.lock().unwrap().len(), 0); // take_snapshot method does not send any message
            assert!(snapshot.data.is_empty());
            assert_eq!(snapshot.state.obs.len(), 0);

            // error
            assert!(src
                .take_snapshot(&hashmap! {"a" => hashmap!{"x" => vec!["i"]}})
                .is_err());
            assert_eq!(src.state.state, state); // state is unchanged
            assert_eq!(record.lock().unwrap().len(), 0); // take_snapshot method does not send any message
        }

        #[rstest]
        fn test_inner(data: DataType) {
            // non-empty
            let mut src = SrcType::from(data.clone());
            let state = src.state.state;
            let record = Arc::new(Mutex::new(Vec::new()));
            let _ = {
                let record = record.clone();
                src.on_change(move |id| {
                    record.lock().unwrap().push(*id);
                })
            };
            assert_eq!(src.state.obs.len(), 1);

            assert_eq!(src.inner(), &data);
            assert_eq!(src.state.state, state); // state is unchanged
            assert_eq!(record.lock().unwrap().len(), 0); // inner method does not send any message

            // empty
            let mut src = SrcType::new();
            let state = src.state.state;
            let _ = {
                let record = record.clone();
                src.on_change(move |id| {
                    record.lock().unwrap().push(*id);
                })
            };

            assert!(src.inner().is_empty());
            assert_eq!(src.state.state, state); // state is unchanged
            assert_eq!(record.lock().unwrap().len(), 0); // inner method does not send any message
        }

        #[rstest]
        fn test_into_inner(data: DataType) {
            // non-empty
            let mut src = SrcType::from(data.clone());
            let record = Arc::new(Mutex::new(Vec::new()));
            let _ = {
                let record = record.clone();
                src.on_change(move |id| {
                    record.lock().unwrap().push(*id);
                })
            };
            assert_eq!(src.state.obs.len(), 1);

            assert_eq!(src.into_inner(), data);
            assert_eq!(record.lock().unwrap().len(), 0); // into_inner method does not send any message

            // empty
            let mut src = SrcType::new();
            let _ = {
                let record = record.clone();
                src.on_change(move |id| {
                    record.lock().unwrap().push(*id);
                })
            };

            assert!(src.into_inner().is_empty());
            assert_eq!(record.lock().unwrap().len(), 0); // into_inner method does not send any message
        }

        #[rstest]
        fn test_len(data: DataType) {
            // non-empty
            let mut src = SrcType::from(data.clone());
            let state = src.state.state;
            let record = Arc::new(Mutex::new(Vec::new()));
            let _ = {
                let record = record.clone();
                src.on_change(move |id| {
                    record.lock().unwrap().push(*id);
                })
            };
            assert_eq!(src.state.obs.len(), 1);

            assert_eq!(src.len(), 12);
            assert_eq!(src.state.state, state); // state is unchanged
            assert_eq!(record.lock().unwrap().len(), 0); // len method does not send any message

            // empty
            let mut src = SrcType::new();
            let state = src.state.state;
            let _ = {
                let record = record.clone();
                src.on_change(move |id| {
                    record.lock().unwrap().push(*id);
                })
            };
            assert_eq!(src.state.obs.len(), 1);

            assert_eq!(src.len(), 0);
            assert_eq!(src.state.state, state); // state is unchanged
            assert_eq!(record.lock().unwrap().len(), 0); // len method does not send any message
        }

        #[rstest]
        fn test_is_empty(data: DataType) {
            // non-empty
            let mut src = SrcType::from(data.clone());
            let state = src.state.state;
            let record = Arc::new(Mutex::new(Vec::new()));
            let _ = {
                let record = record.clone();
                src.on_change(move |id| {
                    record.lock().unwrap().push(*id);
                })
            };
            assert_eq!(src.state.obs.len(), 1);

            assert!(!src.is_empty());
            assert_eq!(src.state.state, state); // state is unchanged
            assert_eq!(record.lock().unwrap().len(), 0); // is_empty method does not send any message

            // empty
            let mut src = SrcType::new();
            let state = src.state.state;
            let _ = {
                let record = record.clone();
                src.on_change(move |id| {
                    record.lock().unwrap().push(*id);
                })
            };
            assert_eq!(src.state.obs.len(), 1);

            assert!(src.is_empty());
            assert_eq!(src.state.state, state); // state is unchanged
            assert_eq!(record.lock().unwrap().len(), 0); // is_empty method does not send any message
        }

        #[rstest]
        fn test_contains_key(data: DataType) {
            // non-empty
            let mut src = SrcType::from(data.clone());
            let state = src.state.state;
            let record = Arc::new(Mutex::new(Vec::new()));
            let _ = {
                let record = record.clone();
                src.on_change(move |id| {
                    record.lock().unwrap().push(*id);
                })
            };
            assert_eq!(src.state.obs.len(), 1);

            assert!(src.contains_key(&"a", &"x", &"i"));
            assert_eq!(src.state.state, state); // state is unchanged
            assert_eq!(record.lock().unwrap().len(), 0); // contains_key method does not send any message
            assert!(!src.contains_key(&"a", &"x", &"k"));
            assert_eq!(src.state.state, state); // state is unchanged
            assert_eq!(record.lock().unwrap().len(), 0); // contains_key method does not send any message
            assert!(src.contains_key(&"c", &"y", &"j"));
            assert_eq!(src.state.state, state); // state is unchanged
            assert_eq!(record.lock().unwrap().len(), 0); // contains_key method does not send any message

            // empty
            let mut src = SrcType::new();
            let state = src.state.state;
            let _ = {
                let record = record.clone();
                src.on_change(move |id| {
                    record.lock().unwrap().push(*id);
                })
            };
            assert_eq!(src.state.obs.len(), 1);

            assert!(!src.contains_key(&"a", &"x", &"i"));
            assert_eq!(src.state.state, state); // state is unchanged
            assert_eq!(record.lock().unwrap().len(), 0); // contains_key method does not send any message
            assert!(!src.contains_key(&"a", &"x", &"k"));
            assert_eq!(src.state.state, state); // state is unchanged
            assert_eq!(record.lock().unwrap().len(), 0); // contains_key method does not send any message
        }

        #[rstest]
        fn test_get(data: DataType) {
            // non-empty
            let mut src = SrcType::from(data.clone());
            let state = src.state.state;
            let record = Arc::new(Mutex::new(Vec::new()));
            let _ = {
                let record = record.clone();
                src.on_change(move |id| {
                    record.lock().unwrap().push(*id);
                })
            };
            assert_eq!(src.state.obs.len(), 1);

            assert_eq!(src.get(&"a", &"x", &"i"), Some(&1));
            assert_eq!(src.state.state, state); // state is unchanged
            assert_eq!(record.lock().unwrap().len(), 0); // get method does not send any message
            assert_eq!(src.get(&"b", &"y", &"j"), Some(&8));
            assert_eq!(src.state.state, state); // state is unchanged
            assert_eq!(record.lock().unwrap().len(), 0); // get method does not send any message
            assert_eq!(src.get(&"c", &"y", &"k"), None);
            assert_eq!(src.state.state, state); // state is unchanged
            assert_eq!(record.lock().unwrap().len(), 0); // get method does not send any message

            // empty
            let mut src = SrcType::new();
            let state = src.state.state;
            let _ = {
                let record = record.clone();
                src.on_change(move |id| {
                    record.lock().unwrap().push(*id);
                })
            };
            assert_eq!(src.state.obs.len(), 1);

            assert_eq!(src.get(&"a", &"x", &"i"), None);
            assert_eq!(src.state.state, state); // state is unchanged
            assert_eq!(record.lock().unwrap().len(), 0); // get method does not send any message
            assert_eq!(src.get(&"b", &"y", &"j"), None);
            assert_eq!(src.state.state, state); // state is unchanged
            assert_eq!(record.lock().unwrap().len(), 0); // get method does not send any message
        }

        #[rstest]
        fn test_insert(data: DataType) {
            let record1 = Arc::new(Mutex::new(Vec::new()));
            let record2 = Arc::new(Mutex::new(Vec::new()));

            // non-empty
            let mut src = SrcType::from(data.clone());
            let state = src.state.state;
            let _1 = {
                let record1 = record1.clone();
                src.on_change(move |id| {
                    record1.lock().unwrap().push(*id);
                })
            };
            let _2 = {
                let record2 = record2.clone();
                src.on_change(move |_| {
                    let mut record = record2.lock().unwrap();
                    let len = record.len();
                    record.push(format!("inserted{}", len));
                })
            };
            assert_eq!(src.state.obs.len(), 2);

            assert_eq!(src.insert("a", "x", "i", 10), Some(1));
            assert_ne!(src.state.state, state);
            assert_eq!(record1.lock().unwrap().len(), 1); // insert method sends a message
            assert_eq!(record1.lock().unwrap()[0], src.state.state);
            assert_eq!(record2.lock().unwrap().len(), 1); // insert method sends a message
            assert_eq!(record2.lock().unwrap()[0], "inserted0");

            assert_eq!(src.insert("c", "y", "k", 20), None);
            assert_ne!(src.state.state, state);
            assert_eq!(record1.lock().unwrap().len(), 2); // insert method sends a message
            assert_eq!(record1.lock().unwrap()[1], src.state.state);
            assert_eq!(record2.lock().unwrap().len(), 2); // insert method sends a message
            assert_eq!(record2.lock().unwrap()[1], "inserted1");

            assert_eq!(src.insert("d", "z", "l", 30), None);
            assert_ne!(src.state.state, state);
            assert_eq!(record1.lock().unwrap().len(), 3); // insert method sends a message
            assert_eq!(record1.lock().unwrap()[2], src.state.state);
            assert_eq!(record2.lock().unwrap().len(), 3); // insert method sends a message
            assert_eq!(record2.lock().unwrap()[2], "inserted2");

            assert_eq!(
                src.data,
                hashmap! {
                    "a" => hashmap!{
                        "x" => hashmap!{
                            "i" => 10,
                            "j" => 2
                        },
                        "y" => hashmap!{
                            "i" => 3,
                            "j" => 4
                        }
                    },
                    "b" => hashmap!{
                        "x" => hashmap!{
                            "i" => 5,
                            "j" => 6
                        },
                        "y" => hashmap!{
                            "i" => 7,
                            "j" => 8
                        }
                    },
                    "c" => hashmap!{
                        "x" => hashmap!{
                            "i" => 9,
                            "j" => 10
                        },
                        "y" => hashmap!{
                            "i" => 11,
                            "j" => 12,
                            "k" => 20
                        }
                    },
                    "d" => hashmap!{
                        "z" => hashmap!{
                            "l" => 30
                        }
                    }
                }
            );

            // empty
            let mut src = SrcType::new();
            let state = src.state.state;
            let _1 = {
                let record1 = record1.clone();
                record1.lock().unwrap().clear();
                src.on_change(move |id| {
                    record1.lock().unwrap().push(*id);
                })
            };
            let _2 = {
                let record2 = record2.clone();
                record2.lock().unwrap().clear();
                src.on_change(move |_| {
                    let mut record = record2.lock().unwrap();
                    let len = record.len();
                    record.push(format!("inserted{}", len));
                })
            };
            assert_eq!(src.state.obs.len(), 2);

            assert_eq!(src.insert("a", "x", "i", 10), None);
            assert_ne!(src.state.state, state);
            assert_eq!(record1.lock().unwrap().len(), 1); // insert method sends a message
            assert_eq!(record2.lock().unwrap().len(), 1); // insert method sends a message
            assert_eq!(record1.lock().unwrap()[0], src.state.state);
            assert_eq!(record2.lock().unwrap()[0], "inserted0");

            assert_eq!(
                src.data,
                hashmap! {"a" => hashmap!{"x" => hashmap!{"i" => 10}}}
            );
        }

        #[rstest]
        fn test_remove(data: DataType) {
            let record1 = Arc::new(Mutex::new(Vec::new()));
            let record2 = Arc::new(Mutex::new(Vec::new()));

            // non-empty
            let mut src = SrcType::from(data.clone());
            let state = src.state.state;
            let _1 = {
                let record1 = record1.clone();
                src.on_change(move |id| {
                    record1.lock().unwrap().push(*id);
                })
            };
            let _2 = {
                let record2 = record2.clone();
                src.on_change(move |_| {
                    let mut record = record2.lock().unwrap();
                    let len = record.len();
                    record.push(format!("removed{}", len));
                })
            };
            assert_eq!(src.state.obs.len(), 2);

            assert_eq!(src.remove(&"a", &"x", &"i"), Some(1));
            assert_ne!(src.state.state, state); // state is changed
            assert_eq!(record1.lock().unwrap().len(), 1); // remove method sends a message
            assert_eq!(record1.lock().unwrap()[0], src.state.state);
            assert_eq!(record2.lock().unwrap().len(), 1); // remove method sends a message
            assert_eq!(record2.lock().unwrap()[0], "removed0");
            let state = src.state.state;

            assert_eq!(src.remove(&"c", &"y", &"j"), Some(12));
            assert_ne!(src.state.state, state); // state is changed
            assert_eq!(record1.lock().unwrap().len(), 2); // remove method sends a message
            assert_eq!(record1.lock().unwrap()[1], src.state.state);
            assert_eq!(record2.lock().unwrap().len(), 2); // remove method sends a message
            assert_eq!(record2.lock().unwrap()[1], "removed1");
            let state = src.state.state;

            assert_eq!(src.remove(&"d", &"z", &"l"), None);
            assert_eq!(src.state.state, state); // state is unchanged
            assert_eq!(record1.lock().unwrap().len(), 2); // remove method does not send any message
            assert_eq!(record2.lock().unwrap().len(), 2); // remove method does not send any message
            assert_eq!(
                src.data,
                hashmap! {
                    "a" => hashmap!{
                        "x" => hashmap!{
                            "j" => 2
                        },
                        "y" => hashmap!{
                            "i" => 3,
                            "j" => 4
                        }
                    },
                    "b" => hashmap!{
                        "x" => hashmap!{
                            "i" => 5,
                            "j" => 6
                        },
                        "y" => hashmap!{
                            "i" => 7,
                            "j" => 8
                        }
                    },
                    "c" => hashmap!{
                        "x" => hashmap!{
                            "i" => 9,
                            "j" => 10
                        },
                        "y" => hashmap! {
                            "i" => 11
                        }
                    }
                }
            );

            // empty
            let mut src = SrcType::new();
            let state = src.state.state;
            let _1 = {
                let record1 = record1.clone();
                record1.lock().unwrap().clear();
                src.on_change(move |id| {
                    record1.lock().unwrap().push(*id);
                })
            };
            let _2 = {
                let record2 = record2.clone();
                record2.lock().unwrap().clear();
                src.on_change(move |_| {
                    let mut record = record2.lock().unwrap();
                    let len = record.len();
                    record.push(format!("removed{}", len));
                })
            };

            assert_eq!(src.remove(&"a", &"x", &"i"), None);
            assert_eq!(src.state.state, state); // state is unchanged
            assert_eq!(record1.lock().unwrap().len(), 0); // remove method does not send any message
            assert_eq!(record2.lock().unwrap().len(), 0); // remove method does not send any message
            assert!(src.data.is_empty());
        }

        #[rstest]
        fn test_remove_entry(data: DataType) {
            let record1 = Arc::new(Mutex::new(Vec::new()));
            let record2 = Arc::new(Mutex::new(Vec::new()));

            // non-empty
            let mut src = SrcType::from(data.clone());
            let state = src.state.state;
            let _1 = {
                let record1 = record1.clone();
                src.on_change(move |id| {
                    record1.lock().unwrap().push(*id);
                })
            };
            let _2 = {
                let record2 = record2.clone();
                src.on_change(move |_| {
                    let mut record = record2.lock().unwrap();
                    let len = record.len();
                    record.push(format!("removed{}", len));
                })
            };
            assert_eq!(src.state.obs.len(), 2);

            assert_eq!(src.remove_entry(&"a", &"x", &"i"), Some(("a", "x", "i", 1)));
            assert_ne!(src.state.state, state); // state is changed
            assert_eq!(record1.lock().unwrap().len(), 1); // remove_entry method sends a message
            assert_eq!(record1.lock().unwrap()[0], src.state.state);
            assert_eq!(record2.lock().unwrap().len(), 1); // remove_entry method sends a message
            assert_eq!(record2.lock().unwrap()[0], "removed0");
            let state = src.state.state;

            assert_eq!(
                src.remove_entry(&"c", &"y", &"j"),
                Some(("c", "y", "j", 12))
            );
            assert_ne!(src.state.state, state); // state is changed
            assert_eq!(record1.lock().unwrap().len(), 2); // remove_entry method sends a message
            assert_eq!(record1.lock().unwrap()[1], src.state.state);
            assert_eq!(record2.lock().unwrap().len(), 2); // remove_entry method sends a message
            assert_eq!(record2.lock().unwrap()[1], "removed1");
            let state = src.state.state;

            assert_eq!(src.remove_entry(&"d", &"z", &"l"), None);
            assert_eq!(src.state.state, state); // state is unchanged
            assert_eq!(record1.lock().unwrap().len(), 2); // remove_entry method does not send any message
            assert_eq!(record2.lock().unwrap().len(), 2); // remove_entry method

            assert_eq!(
                src.data,
                hashmap! {
                    "a" => hashmap!{
                        "x" => hashmap!{
                            "j" => 2
                        },
                        "y" => hashmap!{
                            "i" => 3,
                            "j" => 4
                        }
                    },
                    "b" => hashmap!{
                        "x" => hashmap!{
                            "i" => 5,
                            "j" => 6
                        },
                        "y" => hashmap!{
                            "i" => 7,
                            "j" => 8
                        }
                    },
                    "c" => hashmap!{
                        "x" => hashmap!{
                            "i" => 9,
                            "j" => 10,
                        },
                        "y" => hashmap! {
                            "i" => 11
                        }
                    }
                }
            );

            // empty
            let mut src = SrcType::new();
            let state = src.state.state;
            let _1 = {
                let record1 = record1.clone();
                record1.lock().unwrap().clear();
                src.on_change(move |id| {
                    record1.lock().unwrap().push(*id);
                })
            };
            let _2 = {
                let record2 = record2.clone();
                record2.lock().unwrap().clear();
                src.on_change(move |_| {
                    let mut record = record2.lock().unwrap();
                    let len = record.len();
                    record.push(format!("removed{}", len));
                })
            };

            assert_eq!(src.remove_entry(&"a", &"x", &"i"), None);
            assert_eq!(src.state.state, state); // state is unchanged
            assert_eq!(record1.lock().unwrap().len(), 0); // remove_entry method does not send any message
            assert_eq!(record2.lock().unwrap().len(), 0); // remove_entry method does not send any message
            assert!(src.data.is_empty());
        }

        #[rstest]
        fn test_clear(data: DataType) {
            let record1 = Arc::new(Mutex::new(Vec::new()));
            let record2 = Arc::new(Mutex::new(Vec::new()));

            // non-empty
            let mut src = SrcType::from(data.clone());
            let state = src.state.state;
            let _1 = {
                let record1 = record1.clone();
                src.on_change(move |id| {
                    record1.lock().unwrap().push(*id);
                })
            };
            let _2 = {
                let record2 = record2.clone();
                src.on_change(move |_| {
                    let mut record = record2.lock().unwrap();
                    let len = record.len();
                    record.push(format!("cleared{}", len));
                })
            };
            assert_eq!(src.state.obs.len(), 2);

            src.clear();
            assert_ne!(src.state.state, state); // state is changed
            assert_eq!(record1.lock().unwrap().len(), 1); // clear method sends a message
            assert_eq!(record1.lock().unwrap()[0], src.state.state);
            assert_eq!(record2.lock().unwrap().len(), 1); // clear method sends a message
            assert_eq!(record2.lock().unwrap()[0], "cleared0");
            assert!(src.data.is_empty());
            let state = src.state.state;

            src.clear();
            assert_eq!(src.state.state, state); // state is unchanged
            assert_eq!(record1.lock().unwrap().len(), 1); // clear method does not send any message
            assert_eq!(record2.lock().unwrap().len(), 1); // clear method does not send any message
            assert!(src.data.is_empty());

            // empty
            let mut src = SrcType::new();
            let state = src.state.state;
            let _1 = {
                let record1 = record1.clone();
                record1.lock().unwrap().clear();
                src.on_change(move |id| {
                    record1.lock().unwrap().push(*id);
                })
            };
            let _2 = {
                let record2 = record2.clone();
                record2.lock().unwrap().clear();
                src.on_change(move |_| {
                    let mut record = record2.lock().unwrap();
                    let len = record.len();
                    record.push(format!("cleared{}", len));
                })
            };
            assert_eq!(src.state.obs.len(), 2);

            src.clear();
            assert_eq!(src.state.state, state); // state is unchanged
            assert_eq!(record1.lock().unwrap().len(), 0); // clear method does not send any message
            assert_eq!(record2.lock().unwrap().len(), 0); // clear method does not send any message
            assert!(src.data.is_empty());
        }

        #[rstest]
        fn test_retain(data: DataType) {
            let record1 = Arc::new(Mutex::new(Vec::new()));
            let record2 = Arc::new(Mutex::new(Vec::new()));

            // non-empty
            let mut src = SrcType::from(data.clone());
            let state = src.state.state;
            let _1 = {
                let record1 = record1.clone();
                src.on_change(move |id| {
                    record1.lock().unwrap().push(*id);
                })
            };
            let _2 = {
                let record2 = record2.clone();
                src.on_change(move |_| {
                    let mut record = record2.lock().unwrap();
                    let len = record.len();
                    record.push(format!("retained{}", len));
                })
            };
            assert_eq!(src.state.obs.len(), 2);

            src.retain(|_, _, _, _| true);
            assert_eq!(src.state.state, state); // state is unchanged
            assert_eq!(record1.lock().unwrap().len(), 0); // retain method does not send any message
            assert_eq!(record2.lock().unwrap().len(), 0); // retain method does not send any message
            assert_eq!(src.data, data);

            src.retain(|k1, k2, k3, _| k1 != &"a" && k2 != &"x" && k3 != &"i");
            assert_ne!(src.state.state, state); // state is changed
            assert_eq!(record1.lock().unwrap().len(), 1); // retain method sends a message
            assert_eq!(record1.lock().unwrap()[0], src.state.state);
            assert_eq!(record2.lock().unwrap().len(), 1); // retain method sends a message
            assert_eq!(record2.lock().unwrap()[0], "retained0");
            assert_eq!(
                src.data,
                hashmap! {
                    "b" => hashmap!{
                        "y" => hashmap!{
                            "j" => 8
                        }
                    },
                    "c" => hashmap! {
                        "y" => hashmap! {
                            "j" => 12
                        }
                    }
                }
            );

            // empty
            let mut src = SrcType::new();
            let state = src.state.state;
            let _1 = {
                let record1 = record1.clone();
                record1.lock().unwrap().clear();
                src.on_change(move |id| {
                    record1.lock().unwrap().push(*id);
                })
            };
            let _2 = {
                let record2 = record2.clone();
                record2.lock().unwrap().clear();
                src.on_change(move |_| {
                    let mut record = record2.lock().unwrap();
                    let len = record.len();
                    record.push(format!("retained{}", len));
                })
            };
            assert_eq!(src.state.obs.len(), 2);

            src.retain(|_, _, _, _| false);
            assert_eq!(src.state.state, state); // state is unchanged
            assert_eq!(record1.lock().unwrap().len(), 0); // retain method does not send any message
            assert_eq!(record2.lock().unwrap().len(), 0); // retain method does not send any message
            assert!(src.data.is_empty());
        }
    }
}
