use std::{
    borrow::Borrow,
    collections::HashMap,
    hash::Hash,
    sync::{Arc, Mutex, Weak},
};

use anyhow::anyhow;

use super::{DataSrc, Node, NodeId, NodeInfo, NodeStateId, TakeSnapshot, Tree};

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
    fn accept_state(&self, _: &NodeId, _: &NodeStateId) {}
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
    fn accept_state(&self, _: &NodeId, _: &NodeStateId) {}
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
    pub fn contains_key<Q>(&self, key: &Q) -> bool
    where
        K: Borrow<Q>,
        Q: Eq + Hash + ?Sized,
    {
        self._data().lock().unwrap().contains_key(key)
    }

    pub fn len(&self) -> usize {
        self._data().lock().unwrap().len()
    }

    pub fn insert(&mut self, key: K, value: V) {
        self._data().lock().unwrap().insert(key, value);
        self._info().set_state(NodeStateId::gen());
        self._info().notify_all();
    }

    pub fn remove<Q>(&mut self, key: &Q) -> Option<V>
    where
        K: Borrow<Q>,
        Q: Eq + Hash + ?Sized,
    {
        let res = self._data().lock().unwrap().remove(key);
        if res.is_some() {
            self._info().set_state(NodeStateId::gen());
            self._info().notify_all();
        }
        res
    }

    pub fn remove_entry<Q>(&mut self, key: &Q) -> Option<(K, V)>
    where
        K: Borrow<Q>,
        Q: Eq + Hash + ?Sized,
    {
        let res = self._data().lock().unwrap().remove_entry(key);
        if res.is_some() {
            self._info().set_state(NodeStateId::gen());
            self._info().notify_all();
        }
        res
    }

    pub fn retain(&mut self, f: impl FnMut(&K, &mut V) -> bool) {
        let mut data = self._data().lock().unwrap();
        let orig_len = data.len();
        data.retain(f);
        if orig_len != data.len() {
            self._info().set_state(NodeStateId::gen());
            self._info().notify_all();
        }
    }

    pub fn capacity(&self) -> usize {
        self._data().lock().unwrap().capacity()
    }
}

impl<K: Eq + Hash, V> Extend<(K, V)> for OnMemorySrc<K, V> {
    fn extend<T: IntoIterator<Item = (K, V)>>(&mut self, iter: T) {
        let mut data = self._data().lock().unwrap();
        let orig_len = data.len();
        data.extend(iter);
        if orig_len != data.len() {
            self._info().set_state(NodeStateId::gen());
            self._info().notify_all();
        }
    }
}
