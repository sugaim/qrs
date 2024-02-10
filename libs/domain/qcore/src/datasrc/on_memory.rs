use std::{
    borrow::Borrow,
    collections::HashMap,
    hash::Hash,
    sync::{Arc, Mutex, Weak},
};

use anyhow::anyhow;

use super::{
    node::DataSrc2Args, snapshot::TakeSnapshot3Args, DataSrc, DataSrc3Args, Node, NodeId, NodeInfo,
    NodeStateId, TakeSnapshot, TakeSnapshot2Args, Tree,
};

// -----------------------------------------------------------------------------
// ImmutableOnMemorySrc
//
#[derive(Debug)]
pub struct ImmutableOnMemorySrc<K, V>(Arc<(HashMap<K, V>, NodeInfo)>);

//
// construction
//
impl<K, V> ImmutableOnMemorySrc<K, V> {
    pub fn with_data(desc: impl Into<String>, data: HashMap<K, V>) -> Self {
        Self(Arc::new((data, NodeInfo::new(desc))))
    }
}

impl<K, V> Clone for ImmutableOnMemorySrc<K, V> {
    fn clone(&self) -> Self {
        Self(self.0.clone())
    }
}

//
// methods
//
impl<K, V> ImmutableOnMemorySrc<K, V> {
    fn _info(&self) -> &NodeInfo {
        &self.0 .1
    }
    fn _data(&self) -> &HashMap<K, V> {
        &self.0 .0
    }
}

impl<K: 'static, V: 'static> Node for ImmutableOnMemorySrc<K, V> {
    #[inline]
    fn id(&self) -> NodeId {
        self._info().id()
    }

    #[inline]
    fn tree(&self) -> Tree {
        self._info().make_tree_as_leaf()
    }

    /// This method does nothing because this node is immutable.
    #[inline]
    fn accept_subscriber(&self, _: Weak<dyn Node>) -> NodeStateId {
        self._info().state()
    }

    /// This method does nothing because this node is immutable.
    #[inline]
    fn remove_subscriber(&self, _: &NodeId) {}

    /// This node does not depend on other nodes, so this method does nothing.
    #[inline]
    fn subscribe(&self, _: &NodeId, _: &NodeStateId) {}
}

impl<Q, K, V> DataSrc<Q> for ImmutableOnMemorySrc<K, V>
where
    Q: ?Sized + Eq + Hash,
    K: 'static + Eq + Hash + Borrow<Q>,
    V: 'static + Clone,
{
    type Output = V;
    type Err = anyhow::Error;

    fn req(&self, key: &Q) -> Result<(NodeStateId, Self::Output), Self::Err> {
        let res = self
            ._data()
            .get(key)
            .cloned()
            .ok_or_else(|| anyhow::anyhow!("key not found"))?;
        Ok((self._info().state(), res))
    }
}

impl<Q, K, V> TakeSnapshot<Q> for ImmutableOnMemorySrc<K, V>
where
    Q: ?Sized + Eq + Hash,
    K: 'static + Eq + Hash + Borrow<Q> + Clone,
    V: 'static + Clone,
{
    type SnapShot = ImmutableOnMemorySrc<K, V>;
    type SnapShotErr = Self::Err;

    fn take_snapshot<'a, It>(&self, keys: It) -> Result<Self::SnapShot, Self::Err>
    where
        It: IntoIterator<Item = &'a Q>,
        Q: 'a,
    {
        let data = keys
            .into_iter()
            .map(|k| {
                self._data()
                    .get_key_value(k)
                    .ok_or_else(|| anyhow!("key not found"))
                    .map(|(k, v)| (k.clone(), v.clone()))
            })
            .collect::<Result<_, _>>()?;
        Ok(Self::with_data(self._info().desc().to_string(), data))
    }
}

impl<K: Eq + Hash, V> ImmutableOnMemorySrc<K, V> {
    #[inline]
    pub fn contains_key<Q>(&self, key: &Q) -> bool
    where
        K: Borrow<Q>,
        Q: Eq + Hash + ?Sized,
    {
        self._data().contains_key(key)
    }

    #[inline]
    pub fn len(&self) -> usize {
        self._data().len()
    }

    #[inline]
    pub fn is_empty(&self) -> bool {
        self._data().is_empty()
    }
}

// -----------------------------------------------------------------------------
// ImmutableOnMemorySrc2Args
//

#[derive(Debug)]
pub struct ImmutableOnMemorySrc2Args<K1, K2, V>(Arc<(HashMap<K1, HashMap<K2, V>>, NodeInfo)>);

//
// construction
//
impl<K1, K2, V> ImmutableOnMemorySrc2Args<K1, K2, V>
where
    K1: Eq + Hash,
    K2: Eq + Hash,
{
    #[inline]
    pub fn with_data(desc: impl Into<String>, data: HashMap<(K1, K2), V>) -> Self {
        let mut map = HashMap::new();
        for (k, v) in data {
            map.entry(k.0).or_insert_with(HashMap::new).insert(k.1, v);
        }
        Self(Arc::new((map, NodeInfo::new(desc))))
    }
}

impl<K1, K2, V> Clone for ImmutableOnMemorySrc2Args<K1, K2, V> {
    fn clone(&self) -> Self {
        Self(self.0.clone())
    }
}

//
// methods
//
impl<K1, K2, V> ImmutableOnMemorySrc2Args<K1, K2, V> {
    fn _info(&self) -> &NodeInfo {
        &self.0 .1
    }
    fn _data(&self) -> &HashMap<K1, HashMap<K2, V>> {
        &self.0 .0
    }
}

impl<K1: 'static, K2: 'static, V: 'static> Node for ImmutableOnMemorySrc2Args<K1, K2, V> {
    #[inline]
    fn id(&self) -> NodeId {
        self._info().id()
    }

    #[inline]
    fn tree(&self) -> Tree {
        self._info().make_tree_as_leaf()
    }

    /// This method does nothing because this node is immutable.
    #[inline]
    fn accept_subscriber(&self, _: Weak<dyn Node>) -> NodeStateId {
        self._info().state()
    }

    /// This method does nothing because this node is immutable.
    #[inline]
    fn remove_subscriber(&self, _: &NodeId) {}

    /// This node does not depend on other nodes, so this method does nothing.
    #[inline]
    fn subscribe(&self, _: &NodeId, _: &NodeStateId) {}
}

impl<Q1, Q2, K1, K2, V> DataSrc2Args<Q1, Q2> for ImmutableOnMemorySrc2Args<K1, K2, V>
where
    Q1: ?Sized + Eq + Hash,
    Q2: ?Sized + Eq + Hash,
    K1: 'static + Eq + Hash + Borrow<Q1>,
    K2: 'static + Eq + Hash + Borrow<Q2>,
    V: 'static + Clone,
{
    type Output = V;
    type Err = anyhow::Error;

    fn req(&self, key1: &Q1, key2: &Q2) -> Result<(NodeStateId, Self::Output), Self::Err> {
        let res = self
            ._data()
            .get(key1)
            .and_then(|m| m.get(key2))
            .cloned()
            .ok_or_else(|| anyhow::anyhow!("key not found"))?;
        Ok((self._info().state(), res))
    }
}

impl<Q1, Q2, K1, K2, V> TakeSnapshot2Args<Q1, Q2> for ImmutableOnMemorySrc2Args<K1, K2, V>
where
    Q1: ?Sized + Eq + Hash,
    Q2: ?Sized + Eq + Hash,
    K1: 'static + Eq + Hash + Borrow<Q1> + Clone,
    K2: 'static + Eq + Hash + Borrow<Q2> + Clone,
    V: 'static + Clone,
{
    type SnapShot = ImmutableOnMemorySrc2Args<K1, K2, V>;
    type SnapShotErr = Self::Err;

    fn take_snapshot<'a, It>(&self, keys: It) -> Result<Self::SnapShot, Self::SnapShotErr>
    where
        It: IntoIterator<Item = (&'a Q1, &'a Q2)>,
        Q1: 'a,
        Q2: 'a,
    {
        let mut data = HashMap::new();
        let retrived = keys.into_iter().map(|(k1, k2)| {
            self._data()
                .get_key_value(k1)
                .and_then(|(k1, m)| {
                    m.get_key_value(k2)
                        .map(|(k2, v)| (k1.clone(), k2.clone(), v.clone()))
                })
                .ok_or_else(|| anyhow!("key not found"))
        });
        for item in retrived {
            let (k1, k2, v) = item?;
            data.entry(k1).or_insert_with(HashMap::new).insert(k2, v);
        }
        Ok(Self(Arc::new((
            data,
            NodeInfo::new(self._info().desc().to_string()),
        ))))
    }
}

impl<K1: Eq + Hash, K2: Eq + Hash, V> ImmutableOnMemorySrc2Args<K1, K2, V> {
    #[inline]
    pub fn contains_key<Q1, Q2>(&self, key1: &Q1, key2: &Q2) -> bool
    where
        K1: Borrow<Q1>,
        K2: Borrow<Q2>,
        Q1: Eq + Hash + ?Sized,
        Q2: Eq + Hash + ?Sized,
    {
        let data = self._data();
        data.get(key1).and_then(|m| m.get(key2)).is_some()
    }

    #[inline]
    pub fn len(&self) -> usize {
        self._data().len()
    }

    #[inline]
    pub fn is_empty(&self) -> bool {
        self._data().is_empty()
    }
}

// -----------------------------------------------------------------------------
// ImmutableOnMemorySrc3Args
//
#[derive(Debug)]
pub struct ImmutableOnMemorySrc3Args<K1, K2, K3, V>(
    Arc<(HashMap<K1, HashMap<K2, HashMap<K3, V>>>, NodeInfo)>,
);

//
// construction
//
impl<K1, K2, K3, V> ImmutableOnMemorySrc3Args<K1, K2, K3, V>
where
    K1: Eq + Hash,
    K2: Eq + Hash,
    K3: Eq + Hash,
{
    pub fn with_data(desc: impl Into<String>, data: HashMap<(K1, K2, K3), V>) -> Self {
        let mut map = HashMap::new();
        for (k, v) in data {
            map.entry(k.0)
                .or_insert_with(HashMap::new)
                .entry(k.1)
                .or_insert_with(HashMap::new)
                .insert(k.2, v);
        }
        Self(Arc::new((map, NodeInfo::new(desc))))
    }
}

impl<K1, K2, K3, V> Clone for ImmutableOnMemorySrc3Args<K1, K2, K3, V> {
    fn clone(&self) -> Self {
        Self(self.0.clone())
    }
}

//
// methods
//
impl<K1, K2, K3, V> ImmutableOnMemorySrc3Args<K1, K2, K3, V> {
    fn _info(&self) -> &NodeInfo {
        &self.0 .1
    }
    fn _data(&self) -> &HashMap<K1, HashMap<K2, HashMap<K3, V>>> {
        &self.0 .0
    }
}

impl<K1: 'static, K2: 'static, K3: 'static, V: 'static> Node
    for ImmutableOnMemorySrc3Args<K1, K2, K3, V>
{
    #[inline]
    fn id(&self) -> NodeId {
        self._info().id()
    }

    #[inline]
    fn tree(&self) -> Tree {
        self._info().make_tree_as_leaf()
    }

    /// This method does nothing because this node is immutable.
    #[inline]
    fn accept_subscriber(&self, _: Weak<dyn Node>) -> NodeStateId {
        self._info().state()
    }

    /// This method does nothing because this node is immutable.
    #[inline]
    fn remove_subscriber(&self, _: &NodeId) {}

    /// This node does not depend on other nodes, so this method does nothing.
    #[inline]
    fn subscribe(&self, _: &NodeId, _: &NodeStateId) {}
}

impl<Q1, Q2, Q3, K1, K2, K3, V> DataSrc3Args<Q1, Q2, Q3>
    for ImmutableOnMemorySrc3Args<K1, K2, K3, V>
where
    Q1: ?Sized + Eq + Hash,
    Q2: ?Sized + Eq + Hash,
    Q3: ?Sized + Eq + Hash,
    K1: 'static + Eq + Hash + Borrow<Q1>,
    K2: 'static + Eq + Hash + Borrow<Q2>,
    K3: 'static + Eq + Hash + Borrow<Q3>,
    V: 'static + Clone,
{
    type Output = V;
    type Err = anyhow::Error;

    fn req(
        &self,
        key1: &Q1,
        key2: &Q2,
        key3: &Q3,
    ) -> Result<(NodeStateId, Self::Output), Self::Err> {
        let res = self
            ._data()
            .get(key1)
            .and_then(|m1| m1.get(key2).and_then(|m2| m2.get(key3)))
            .cloned()
            .ok_or_else(|| anyhow::anyhow!("key not found"))?;
        Ok((self._info().state(), res))
    }
}

impl<Q1, Q2, Q3, K1, K2, K3, V> TakeSnapshot3Args<Q1, Q2, Q3>
    for ImmutableOnMemorySrc3Args<K1, K2, K3, V>
where
    Q1: ?Sized + Eq + Hash,
    Q2: ?Sized + Eq + Hash,
    Q3: ?Sized + Eq + Hash,
    K1: 'static + Eq + Hash + Borrow<Q1> + Clone,
    K2: 'static + Eq + Hash + Borrow<Q2> + Clone,
    K3: 'static + Eq + Hash + Borrow<Q3> + Clone,
    V: 'static + Clone,
{
    type SnapShot = ImmutableOnMemorySrc3Args<K1, K2, K3, V>;
    type SnapShotErr = Self::Err;

    fn take_snapshot<'a, It>(&self, keys: It) -> Result<Self::SnapShot, Self::SnapShotErr>
    where
        It: IntoIterator<Item = (&'a Q1, &'a Q2, &'a Q3)>,
        Q1: 'a,
        Q2: 'a,
        Q3: 'a,
    {
        let mut data = HashMap::new();
        let retrived = keys.into_iter().map(|(k1, k2, k3)| {
            self._data()
                .get_key_value(k1)
                .and_then(|(k1, m1)| {
                    m1.get_key_value(k2).and_then(|(k2, m2)| {
                        m2.get_key_value(k3)
                            .map(|(k3, v)| (k1.clone(), k2.clone(), k3.clone(), v.clone()))
                    })
                })
                .ok_or_else(|| anyhow!("key not found"))
        });
        for item in retrived {
            let (k1, k2, k3, v) = item?;
            data.entry(k1)
                .or_insert_with(HashMap::new)
                .entry(k2)
                .or_insert_with(HashMap::new)
                .insert(k3, v);
        }
        Ok(Self(Arc::new((
            data,
            NodeInfo::new(self._info().desc().to_string()),
        ))))
    }
}

impl<K1: Eq + Hash, K2: Eq + Hash, K3: Eq + Hash, V> ImmutableOnMemorySrc3Args<K1, K2, K3, V> {
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
        let data = self._data();
        data.get(key1)
            .and_then(|m1| m1.get(key2).and_then(|m2| m2.get(key3)))
            .is_some()
    }

    #[inline]
    pub fn len(&self) -> usize {
        self._data().len()
    }

    #[inline]
    pub fn is_empty(&self) -> bool {
        self._data().is_empty()
    }
}

// -----------------------------------------------------------------------------
// MutableOnMemorySrc
//
#[derive(Debug)]
pub struct OnMemorySrc<K, V>(Arc<(Mutex<HashMap<K, V>>, NodeInfo)>);

//
// construction
//
impl<K, V> OnMemorySrc<K, V> {
    pub fn new(desc: impl Into<String>) -> Self {
        Self(Arc::new((Mutex::new(HashMap::new()), NodeInfo::new(desc))))
    }

    pub fn with_data(desc: impl Into<String>, data: HashMap<K, V>) -> Self {
        Self(Arc::new((Mutex::new(data), NodeInfo::new(desc))))
    }

    pub fn with_capacity(desc: impl Into<String>, capacity: usize) -> Self {
        Self::with_data(desc, HashMap::with_capacity(capacity))
    }
}

impl<K, V> Clone for OnMemorySrc<K, V> {
    fn clone(&self) -> Self {
        Self(self.0.clone())
    }
}

//
// methods
//
impl<K, V> OnMemorySrc<K, V> {
    // private methods to access inner data
    fn _info(&self) -> &NodeInfo {
        &self.0 .1
    }
    fn _data(&self) -> &Mutex<HashMap<K, V>> {
        &self.0 .0
    }
}

impl<K: 'static, V: 'static> Node for OnMemorySrc<K, V> {
    #[inline]
    fn id(&self) -> NodeId {
        self._info().id()
    }

    #[inline]
    fn tree(&self) -> Tree {
        self._info().make_tree_as_leaf()
    }

    #[inline]
    fn accept_subscriber(&self, subscriber: Weak<dyn Node>) -> NodeStateId {
        self._info().accept_subscriber(subscriber)
    }

    #[inline]
    fn remove_subscriber(&self, subscriber: &NodeId) {
        self._info().remove_subscriber(subscriber);
    }

    /// This node does not depend on other nodes, so this method does nothing.
    #[inline]
    fn subscribe(&self, _: &NodeId, _: &NodeStateId) {}
}

impl<Q, K, V> DataSrc<Q> for OnMemorySrc<K, V>
where
    Q: ?Sized + Eq + Hash,
    K: 'static + Eq + Hash + Borrow<Q>,
    V: 'static + Clone,
{
    type Output = V;
    type Err = anyhow::Error;

    fn req(&self, key: &Q) -> Result<(NodeStateId, Self::Output), Self::Err> {
        let res = self
            ._data()
            .lock()
            .unwrap()
            .get(key)
            .cloned()
            .ok_or_else(|| anyhow::anyhow!("key not found"))?;
        Ok((self._info().state(), res))
    }
}

impl<Q, K, V> TakeSnapshot<Q> for OnMemorySrc<K, V>
where
    Q: ?Sized + Eq + Hash,
    K: 'static + Eq + Hash + Borrow<Q> + Clone,
    V: 'static + Clone,
{
    type SnapShot = ImmutableOnMemorySrc<K, V>;
    type SnapShotErr = Self::Err;

    fn take_snapshot<'a, It>(&self, keys: It) -> Result<Self::SnapShot, Self::Err>
    where
        It: IntoIterator<Item = &'a Q>,
        Q: 'a,
    {
        let data = self._data().lock().unwrap();
        let data = keys
            .into_iter()
            .map(|k| {
                data.get_key_value(k)
                    .ok_or_else(|| anyhow!("key not found"))
                    .map(|(k, v)| (k.clone(), v.clone()))
            })
            .collect::<Result<_, _>>()?;
        Ok(ImmutableOnMemorySrc::with_data(
            self._info().desc().to_string(),
            data,
        ))
    }
}

impl<K: Eq + Hash, V> OnMemorySrc<K, V> {
    #[inline]
    pub fn contains_key<Q>(&self, key: &Q) -> bool
    where
        K: Borrow<Q>,
        Q: Eq + Hash + ?Sized,
    {
        self._data().lock().unwrap().contains_key(key)
    }

    #[inline]
    pub fn len(&self) -> usize {
        self._data().lock().unwrap().len()
    }

    #[inline]
    pub fn is_empty(&self) -> bool {
        self._data().lock().unwrap().is_empty()
    }

    pub fn insert(&mut self, key: K, value: V) {
        self._data().lock().unwrap().insert(key, value);
        self._info().set_state(NodeStateId::gen());
    }

    pub fn remove<Q>(&mut self, key: &Q) -> Option<V>
    where
        K: Borrow<Q>,
        Q: Eq + Hash + ?Sized,
    {
        let res = self._data().lock().unwrap().remove(key);
        if res.is_some() {
            self._info().set_state(NodeStateId::gen());
        }
        res
    }

    pub fn retain(&mut self, f: impl FnMut(&K, &mut V) -> bool) {
        let mut data = self._data().lock().unwrap();
        let orig_len = data.len();
        data.retain(f);
        if orig_len != data.len() {
            self._info().set_state(NodeStateId::gen());
        }
    }

    #[inline]
    pub fn capacity(&self) -> usize {
        self._data().lock().unwrap().capacity()
    }

    #[inline]
    pub fn clear(&mut self) {
        let mut data = self._data().lock().unwrap();
        if !data.is_empty() {
            data.clear();
            self._info().set_state(NodeStateId::gen());
        }
    }
}

impl<K: Eq + Hash, V> Extend<(K, V)> for OnMemorySrc<K, V> {
    fn extend<T: IntoIterator<Item = (K, V)>>(&mut self, iter: T) {
        let mut data = self._data().lock().unwrap();
        let orig_len = data.len();
        data.extend(iter);
        if orig_len != data.len() {
            self._info().set_state(NodeStateId::gen());
        }
    }
}

// -----------------------------------------------------------------------------
// OnMemorySrc2Args
//
#[derive(Debug)]
pub struct OnMemorySrc2Args<K1, K2, V>(Arc<(Mutex<HashMap<K1, HashMap<K2, V>>>, NodeInfo)>);

//
// construction
//
impl<K1, K2, V> OnMemorySrc2Args<K1, K2, V>
where
    K1: Eq + Hash,
    K2: Eq + Hash,
{
    #[inline]
    pub fn new(desc: impl Into<String>) -> Self {
        Self(Arc::new((Mutex::new(HashMap::new()), NodeInfo::new(desc))))
    }

    #[inline]
    pub fn with_data(desc: impl Into<String>, data: HashMap<(K1, K2), V>) -> Self {
        let mut map = HashMap::new();
        for (k, v) in data {
            map.entry(k.0).or_insert_with(HashMap::new).insert(k.1, v);
        }
        Self(Arc::new((Mutex::new(map), NodeInfo::new(desc))))
    }

    #[inline]
    pub fn with_capacity(desc: impl Into<String>, capacity: usize) -> Self {
        Self::with_data(desc, HashMap::with_capacity(capacity))
    }
}

impl<K1, K2, V> Clone for OnMemorySrc2Args<K1, K2, V> {
    #[inline]
    fn clone(&self) -> Self {
        Self(self.0.clone())
    }
}

//
// methods
//
impl<K1, K2, V> OnMemorySrc2Args<K1, K2, V> {
    #[inline]
    fn _info(&self) -> &NodeInfo {
        &self.0 .1
    }
    #[inline]
    fn _data(&self) -> &Mutex<HashMap<K1, HashMap<K2, V>>> {
        &self.0 .0
    }
}

impl<K1: 'static, K2: 'static, V: 'static> Node for OnMemorySrc2Args<K1, K2, V> {
    #[inline]
    fn id(&self) -> NodeId {
        self._info().id()
    }

    #[inline]
    fn tree(&self) -> Tree {
        self._info().make_tree_as_leaf()
    }

    #[inline]
    fn accept_subscriber(&self, subscriber: Weak<dyn Node>) -> NodeStateId {
        self._info().accept_subscriber(subscriber)
    }

    #[inline]
    fn remove_subscriber(&self, subscriber: &NodeId) {
        self._info().remove_subscriber(subscriber);
    }

    /// This node does not depend on other nodes, so this method does nothing.
    #[inline]
    fn subscribe(&self, _: &NodeId, _: &NodeStateId) {}
}

impl<Q1, Q2, K1, K2, V> DataSrc2Args<Q1, Q2> for OnMemorySrc2Args<K1, K2, V>
where
    Q1: ?Sized + Eq + Hash,
    Q2: ?Sized + Eq + Hash,
    K1: 'static + Eq + Hash + Borrow<Q1>,
    K2: 'static + Eq + Hash + Borrow<Q2>,
    V: 'static + Clone,
{
    type Output = V;
    type Err = anyhow::Error;

    fn req(&self, key1: &Q1, key2: &Q2) -> Result<(NodeStateId, Self::Output), Self::Err> {
        let res = self
            ._data()
            .lock()
            .unwrap()
            .get(key1)
            .and_then(|m| m.get(key2))
            .cloned()
            .ok_or_else(|| anyhow::anyhow!("key not found"))?;
        Ok((self._info().state(), res))
    }
}

impl<Q1, Q2, K1, K2, V> TakeSnapshot2Args<Q1, Q2> for OnMemorySrc2Args<K1, K2, V>
where
    Q1: ?Sized + Eq + Hash,
    Q2: ?Sized + Eq + Hash,
    K1: 'static + Eq + Hash + Borrow<Q1> + Clone,
    K2: 'static + Eq + Hash + Borrow<Q2> + Clone,
    V: 'static + Clone,
{
    type SnapShot = ImmutableOnMemorySrc2Args<K1, K2, V>;
    type SnapShotErr = Self::Err;

    fn take_snapshot<'a, It>(&self, keys: It) -> Result<Self::SnapShot, Self::SnapShotErr>
    where
        It: IntoIterator<Item = (&'a Q1, &'a Q2)>,
        Q1: 'a,
        Q2: 'a,
    {
        let mut data = HashMap::new();
        let retrived = keys.into_iter().map(|(k1, k2)| {
            self._data()
                .lock()
                .unwrap()
                .get_key_value(k1)
                .and_then(|(k1, m)| {
                    m.get_key_value(k2)
                        .map(|(k2, v)| (k1.clone(), k2.clone(), v.clone()))
                })
                .ok_or_else(|| anyhow!("key not found"))
        });
        for item in retrived {
            let (k1, k2, v) = item?;
            data.entry(k1).or_insert_with(HashMap::new).insert(k2, v);
        }
        Ok(ImmutableOnMemorySrc2Args(Arc::new((
            data,
            NodeInfo::new(self._info().desc().to_string()),
        ))))
    }
}

impl<K1: Eq + Hash, K2: Eq + Hash, V> OnMemorySrc2Args<K1, K2, V> {
    #[inline]
    pub fn contains_key<Q1, Q2>(&self, key1: &Q1, key2: &Q2) -> bool
    where
        K1: Borrow<Q1>,
        K2: Borrow<Q2>,
        Q1: Eq + Hash + ?Sized,
        Q2: Eq + Hash + ?Sized,
    {
        let data = self._data().lock().unwrap();
        data.get(key1).and_then(|m| m.get(key2)).is_some()
    }

    #[inline]
    pub fn len(&self) -> usize {
        self._data().lock().unwrap().len()
    }

    #[inline]
    pub fn is_empty(&self) -> bool {
        self._data().lock().unwrap().is_empty()
    }

    pub fn insert(&mut self, key1: K1, key2: K2, value: V) {
        let mut data = self._data().lock().unwrap();
        data.entry(key1)
            .or_insert_with(HashMap::new)
            .insert(key2, value);
        self._info().set_state(NodeStateId::gen());
    }

    pub fn remove<Q1, Q2>(&mut self, key1: &Q1, key2: &Q2) -> Option<V>
    where
        K1: Borrow<Q1>,
        K2: Borrow<Q2>,
        Q1: Eq + Hash + ?Sized,
        Q2: Eq + Hash + ?Sized,
    {
        let mut data = self._data().lock().unwrap();
        let res = data.get_mut(key1).and_then(|m| m.remove(key2));
        if res.is_some() {
            data.retain(|_, m| !m.is_empty());
            self._info().set_state(NodeStateId::gen());
        }
        res
    }

    pub fn retain(&mut self, mut f: impl FnMut(&K1, &K2, &mut V) -> bool) {
        let mut data = self._data().lock().unwrap();
        let mut has_modified = false;
        data.iter_mut().for_each(|(k1, m)| {
            let orig_len = m.len();
            m.retain(|k2, v| f(k1, k2, v));
            if orig_len != m.len() {
                has_modified = true;
            }
        });
        data.retain(|_, m| !m.is_empty());
        if has_modified {
            self._info().set_state(NodeStateId::gen());
        }
    }

    #[inline]
    pub fn capacity(&self) -> usize {
        self._data().lock().unwrap().capacity()
    }

    #[inline]
    pub fn clear(&mut self) {
        let mut data = self._data().lock().unwrap();
        if !data.is_empty() {
            data.clear();
            self._info().set_state(NodeStateId::gen());
        }
    }
}

// -----------------------------------------------------------------------------
// OnMemorySrc3Args
//
#[derive(Debug)]
pub struct OnMemorySrc3Args<K1, K2, K3, V>(
    Arc<(Mutex<HashMap<K1, HashMap<K2, HashMap<K3, V>>>>, NodeInfo)>,
);

//
// construction
//
impl<K1, K2, K3, V> OnMemorySrc3Args<K1, K2, K3, V>
where
    K1: Eq + Hash,
    K2: Eq + Hash,
    K3: Eq + Hash,
{
    #[inline]
    pub fn new(desc: impl Into<String>) -> Self {
        Self(Arc::new((Mutex::new(HashMap::new()), NodeInfo::new(desc))))
    }

    pub fn with_data(desc: impl Into<String>, data: HashMap<(K1, K2, K3), V>) -> Self {
        let mut map = HashMap::new();
        for (k, v) in data {
            map.entry(k.0)
                .or_insert_with(HashMap::new)
                .entry(k.1)
                .or_insert_with(HashMap::new)
                .insert(k.2, v);
        }
        Self(Arc::new((Mutex::new(map), NodeInfo::new(desc))))
    }

    #[inline]
    pub fn with_capacity(desc: impl Into<String>, capacity: usize) -> Self {
        Self::with_data(desc, HashMap::with_capacity(capacity))
    }
}

impl<K1, K2, K3, V> Clone for OnMemorySrc3Args<K1, K2, K3, V> {
    #[inline]
    fn clone(&self) -> Self {
        Self(self.0.clone())
    }
}

//
// methods
//
impl<K1, K2, K3, V> OnMemorySrc3Args<K1, K2, K3, V> {
    #[inline]
    fn _info(&self) -> &NodeInfo {
        &self.0 .1
    }
    #[inline]
    fn _data(&self) -> &Mutex<HashMap<K1, HashMap<K2, HashMap<K3, V>>>> {
        &self.0 .0
    }
}

impl<K1: 'static, K2: 'static, K3: 'static, V: 'static> Node for OnMemorySrc3Args<K1, K2, K3, V> {
    #[inline]
    fn id(&self) -> NodeId {
        self._info().id()
    }

    #[inline]
    fn tree(&self) -> Tree {
        self._info().make_tree_as_leaf()
    }

    #[inline]
    fn accept_subscriber(&self, subscriber: Weak<dyn Node>) -> NodeStateId {
        self._info().accept_subscriber(subscriber)
    }

    #[inline]
    fn remove_subscriber(&self, subscriber: &NodeId) {
        self._info().remove_subscriber(subscriber);
    }

    /// This node does not depend on other nodes, so this method does nothing.
    #[inline]
    fn subscribe(&self, _: &NodeId, _: &NodeStateId) {}
}

impl<Q1, Q2, Q3, K1, K2, K3, V> DataSrc3Args<Q1, Q2, Q3> for OnMemorySrc3Args<K1, K2, K3, V>
where
    Q1: ?Sized + Eq + Hash,
    Q2: ?Sized + Eq + Hash,
    Q3: ?Sized + Eq + Hash,
    K1: 'static + Eq + Hash + Borrow<Q1>,
    K2: 'static + Eq + Hash + Borrow<Q2>,
    K3: 'static + Eq + Hash + Borrow<Q3>,
    V: 'static + Clone,
{
    type Output = V;
    type Err = anyhow::Error;

    fn req(
        &self,
        key1: &Q1,
        key2: &Q2,
        key3: &Q3,
    ) -> Result<(NodeStateId, Self::Output), Self::Err> {
        let res = self
            ._data()
            .lock()
            .unwrap()
            .get(key1)
            .and_then(|m1| m1.get(key2).and_then(|m2| m2.get(key3)))
            .cloned()
            .ok_or_else(|| anyhow::anyhow!("key not found"))?;
        Ok((self._info().state(), res))
    }
}

impl<Q1, Q2, Q3, K1, K2, K3, V> TakeSnapshot3Args<Q1, Q2, Q3> for OnMemorySrc3Args<K1, K2, K3, V>
where
    Q1: ?Sized + Eq + Hash,
    Q2: ?Sized + Eq + Hash,
    Q3: ?Sized + Eq + Hash,
    K1: 'static + Eq + Hash + Borrow<Q1> + Clone,
    K2: 'static + Eq + Hash + Borrow<Q2> + Clone,
    K3: 'static + Eq + Hash + Borrow<Q3> + Clone,
    V: 'static + Clone,
{
    type SnapShot = ImmutableOnMemorySrc3Args<K1, K2, K3, V>;
    type SnapShotErr = Self::Err;

    fn take_snapshot<'a, It>(&self, keys: It) -> Result<Self::SnapShot, Self::SnapShotErr>
    where
        It: IntoIterator<Item = (&'a Q1, &'a Q2, &'a Q3)>,
        Q1: 'a,
        Q2: 'a,
        Q3: 'a,
    {
        let mut data = HashMap::new();
        let retrived = keys.into_iter().map(|(k1, k2, k3)| {
            self._data()
                .lock()
                .unwrap()
                .get_key_value(k1)
                .and_then(|(k1, m1)| {
                    m1.get_key_value(k2).and_then(|(k2, m2)| {
                        m2.get_key_value(k3)
                            .map(|(k3, v)| (k1.clone(), k2.clone(), k3.clone(), v.clone()))
                    })
                })
                .ok_or_else(|| anyhow!("key not found"))
        });
        for item in retrived {
            let (k1, k2, k3, v) = item?;
            data.entry(k1)
                .or_insert_with(HashMap::new)
                .entry(k2)
                .or_insert_with(HashMap::new)
                .insert(k3, v);
        }
        Ok(ImmutableOnMemorySrc3Args(Arc::new((
            data,
            NodeInfo::new(self._info().desc().to_string()),
        ))))
    }
}

impl<K1: Eq + Hash, K2: Eq + Hash, K3: Eq + Hash, V> OnMemorySrc3Args<K1, K2, K3, V> {
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
        let data = self._data().lock().unwrap();
        data.get(key1)
            .and_then(|m1| m1.get(key2).and_then(|m2| m2.get(key3)))
            .is_some()
    }

    #[inline]
    pub fn len(&self) -> usize {
        self._data().lock().unwrap().len()
    }

    #[inline]
    pub fn is_empty(&self) -> bool {
        self._data().lock().unwrap().is_empty()
    }

    pub fn insert(&mut self, key1: K1, key2: K2, key3: K3, value: V) {
        let mut data = self._data().lock().unwrap();
        data.entry(key1)
            .or_insert_with(HashMap::new)
            .entry(key2)
            .or_insert_with(HashMap::new)
            .insert(key3, value);
        self._info().set_state(NodeStateId::gen());
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
        let mut data = self._data().lock().unwrap();
        let res = data
            .get_mut(key1)
            .and_then(|m1| m1.get_mut(key2).and_then(|m2| m2.remove(key3)));
        if res.is_some() {
            data.iter_mut()
                .for_each(|(_, m1)| m1.retain(|_, m2| !m2.is_empty()));
            data.retain(|_, m1| !m1.is_empty());
            self._info().set_state(NodeStateId::gen());
        }
        res
    }

    pub fn retain(&mut self, mut f: impl FnMut(&K1, &K2, &K3, &mut V) -> bool) {
        let mut data = self._data().lock().unwrap();
        let mut has_modified = false;
        data.iter_mut().for_each(|(k1, m1)| {
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
            data.retain(|_, m1| !m1.is_empty());
            self._info().set_state(NodeStateId::gen());
        }
    }

    #[inline]
    pub fn capacity(&self) -> usize {
        self._data().lock().unwrap().capacity()
    }

    #[inline]
    pub fn clear(&mut self) {
        let mut data = self._data().lock().unwrap();
        if !data.is_empty() {
            data.clear();
            self._info().set_state(NodeStateId::gen());
        }
    }
}
