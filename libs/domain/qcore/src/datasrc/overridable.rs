use std::{borrow::Borrow, collections::HashMap, hash::Hash, sync::Arc, vec};

use qcore_derive::Node;

use super::{
    _private::_UnaryPassThroughNode, node::DataSrc2Args, snapshot::TakeSnapshot3Args, DataSrc,
    DataSrc3Args, Node, NodeStateId, TakeSnapshot, TakeSnapshot2Args,
};

// -----------------------------------------------------------------------------
// _Node
//
mod _node {
    use std::sync::{Arc, Mutex};

    use maplit::btreeset;

    use crate::datasrc::{Node, NodeId, NodeInfo, NodeStateId, Tree};

    #[derive(Debug)]
    pub(super) struct _Node<S, L> {
        pub src: S,

        // as node state id, we use the combined value of the current override layer state
        // and the downstream state id.
        info: NodeInfo,

        // state ids for override layers. To represent the layer structure,
        // we use stack(vec) to store the state ids.
        // the first element is for id when no override layer is applied.
        // the second element is for id when the first override layer is applied and so on.
        // hence, the last element is for the id when all override layers are applied
        // and the current state of the node.
        // if the top of override layer is popped, the state id is also popped.
        override_state: Mutex<Vec<NodeStateId>>,

        // override layers
        layers: Mutex<Vec<L>>,
    }

    //
    // construction
    //
    impl<S: Node, L: 'static> _Node<S, L> {
        pub fn new(desc: impl Into<String>, src: S) -> Arc<Self> {
            let res = Arc::new(Self {
                src,
                info: NodeInfo::new(desc),
                override_state: Mutex::new(vec![NodeStateId::gen()]),
                layers: Mutex::new(Vec::new()),
            });

            // state id of this node = state id of downstream node ^ state id of override layer
            let subsc = Arc::downgrade(&res);
            let downstream_state = res.src.accept_subscriber(subsc);
            let state = downstream_state ^ *res.override_state.lock().unwrap().last().unwrap();
            res.info.set_state(state);
            res
        }
    }

    //
    // methods
    //
    impl<S, L> _Node<S, L> {
        /// Get the state id of the node.
        pub fn state(&self) -> NodeStateId {
            self.info.state()
        }

        /// Desc
        pub fn desc(&self) -> &str {
            self.info.desc()
        }

        /// Pop the top override layer.
        ///
        /// The state id of the node after the layer is popped is also returned.
        pub fn pop(&self) -> Option<(NodeStateId, L)> {
            let popped = self.layers.lock().unwrap().pop();
            if popped.is_none() {
                return None;
            }
            let prev_state = self.override_state.lock().unwrap().pop().unwrap();
            let new_state = self.override_state.lock().unwrap().last().unwrap().clone();
            let mut node_state = self.info.state();

            // remove prev state(see bitxor property) and reflect new state
            node_state ^= prev_state;
            node_state ^= new_state;
            self.info.set_state(node_state);
            Some((node_state, popped.unwrap()))
        }

        /// Push a new override layer.
        ///
        /// The state id of the node after the layer is pushed is returned.
        pub fn push(&self, layer: L) -> NodeStateId {
            let prev_state = self.override_state.lock().unwrap().last().unwrap().clone();
            let new_state = NodeStateId::gen();
            let mut node_state = self.info.state();

            // remove prev state(see bitxor property) and reflect new state
            node_state ^= prev_state;
            node_state ^= new_state;
            self.info.set_state(node_state);
            self.override_state.lock().unwrap().push(new_state);
            self.layers.lock().unwrap().push(layer);
            node_state
        }

        pub fn clear(&self) -> NodeStateId {
            let mut layers = self.layers.lock().unwrap();
            if layers.is_empty() {
                return self.info.state();
            }
            let mut states = self.override_state.lock().unwrap();
            layers.clear();

            let prev_state = states.pop().unwrap();
            let new_state = states.first().unwrap().clone();
            states.truncate(1);

            let mut node_state = self.info.state();
            node_state ^= prev_state;
            node_state ^= new_state;
            self.info.set_state(node_state);
            node_state
        }

        /// Get a value from the override layers.
        /// The value is found from the top layer to the bottom layer.
        pub fn get_from_top<O>(&self, f: impl Fn(&L) -> Option<O>) -> Option<O> {
            self.layers.lock().unwrap().iter().rev().find_map(f)
        }

        /// Get a value from the override layers.
        /// The value is found from the bottom layer to the top layer.
        pub fn get_from_bottom<O>(&self, f: impl Fn(&L) -> Option<O>) -> Option<O> {
            self.layers.lock().unwrap().iter().find_map(f)
        }

        #[inline]
        pub fn num_layers(&self) -> usize {
            self.layers.lock().unwrap().len()
        }

        #[inline]
        pub fn extend(&self, layers: Vec<L>) {
            layers.into_iter().for_each(|layer| {
                self.push(layer);
            });
        }
    }

    impl<S: Node, L: 'static> Node for _Node<S, L> {
        #[inline]
        fn id(&self) -> NodeId {
            self.info.id()
        }

        #[inline]
        fn tree(&self) -> Tree {
            Tree::Branch {
                desc: self.info.desc().to_owned(),
                id: self.id(),
                state: self.info.state(),
                children: btreeset![self.src.tree()],
            }
        }

        #[inline]
        fn accept_subscriber(&self, subscriber: std::sync::Weak<dyn Node>) -> NodeStateId {
            self.info.accept_subscriber(subscriber)
        }

        #[inline]
        fn remove_subscriber(&self, subscriber: &NodeId) {
            self.info.remove_subscriber(subscriber)
        }

        #[inline]
        fn subscribe(&self, publisher: &NodeId, state: &NodeStateId) {
            if publisher != &self.src.id() {
                return;
            }
            let new_state = state ^ *self.override_state.lock().unwrap().last().unwrap();
            self.info.set_state(new_state);
        }
    }
}

// -----------------------------------------------------------------------------
// Overriden
//
#[derive(Debug, Node)]
#[node(transparent = "core")]
pub struct Overriden<S, K, V> {
    core: Arc<_UnaryPassThroughNode<S>>,
    layer: Arc<HashMap<K, V>>,
}

//
// construction
//
impl<S: Node, K, V> Overriden<S, K, V> {
    #[inline]
    pub fn with_layer(desc: impl Into<String>, src: S, layer: HashMap<K, V>) -> Self {
        Self {
            core: _UnaryPassThroughNode::new(src, desc),
            layer: Arc::new(layer),
        }
    }
}

impl<S, K, V> Clone for Overriden<S, K, V> {
    #[inline]
    fn clone(&self) -> Self {
        Self {
            core: self.core.clone(),
            layer: self.layer.clone(),
        }
    }
}

//
// methods
//
impl<S, K, V> Overriden<S, K, V> {
    #[inline]
    pub fn downstream(&self) -> &S {
        &self.core.src
    }
}

impl<Q, S, K, V> DataSrc<Q> for Overriden<S, K, V>
where
    Q: ?Sized + Eq + Hash,
    S: DataSrc<Q>,
    K: 'static + Eq + Hash + Borrow<Q>,
    V: 'static + Clone + Into<S::Output>,
{
    type Output = S::Output;
    type Err = S::Err;

    #[inline]
    fn req(&self, key: &Q) -> Result<(super::NodeStateId, Self::Output), Self::Err> {
        if let Some(val) = self.layer.get(key) {
            return Ok((self.core.state(), val.clone().into()));
        }
        self.core.src.req(key)
    }
}

impl<Q, S, K, V> TakeSnapshot<Q> for Overriden<S, K, V>
where
    Q: ?Sized + Eq + Hash,
    S: TakeSnapshot<Q>,
    K: 'static + Eq + Hash + Borrow<Q> + Clone,
    V: 'static + Clone + Into<S::Output>,
{
    type SnapShot = Overriden<S::SnapShot, K, V>;
    type SnapShotErr = S::SnapShotErr;

    fn take_snapshot<'a, It>(&self, keys: It) -> Result<Self::SnapShot, Self::SnapShotErr>
    where
        It: IntoIterator<Item = &'a Q>,
        Q: 'a,
    {
        let items = keys.into_iter().collect::<Vec<_>>();
        let snapshot = self.core.src.take_snapshot(items.iter().map(|q| *q))?;

        let contained = items.iter().filter_map(|k| self.layer.get_key_value(k));
        let layer = contained.map(|(k, v)| (k.clone(), v.clone())).collect();
        Ok(Overriden::with_layer(self.core.desc(), snapshot, layer))
    }
}

// -----------------------------------------------------------------------------
// Overridable
//
#[derive(Debug, Node)]
#[node(transparent = "core")]
pub struct Overridable<S, K, V> {
    core: Arc<_node::_Node<S, HashMap<K, V>>>,
}

//
// construction
//
impl<S: Node, K: 'static, V: 'static> Overridable<S, K, V> {
    #[inline]
    pub fn _new(desc: impl Into<String>, src: S, layers: Vec<HashMap<K, V>>) -> Self {
        let core = _node::_Node::new(desc, src);
        core.extend(layers);
        Self { core }
    }
    #[inline]
    pub fn with_layer(desc: impl Into<String>, src: S, layer: HashMap<K, V>) -> Self {
        Self::_new(desc, src, vec![layer])
    }
    #[inline]
    pub fn new(desc: impl Into<String>, src: S) -> Self {
        Self::_new(desc, src, Vec::new())
    }
}

impl<S, K, V> Clone for Overridable<S, K, V> {
    #[inline]
    fn clone(&self) -> Self {
        Self {
            core: self.core.clone(),
        }
    }
}

//
// methods
//
impl<S, K, V> Overridable<S, K, V> {
    #[inline]
    pub fn downstream(&self) -> &S {
        &self.core.src
    }
}

impl<Q, S, K, V> DataSrc<Q> for Overridable<S, K, V>
where
    Q: ?Sized + Eq + Hash,
    S: DataSrc<Q>,
    K: 'static + Eq + Hash + Borrow<Q>,
    V: 'static + Clone + Into<S::Output>,
{
    type Output = S::Output;
    type Err = S::Err;

    fn req(&self, key: &Q) -> Result<(super::NodeStateId, Self::Output), Self::Err> {
        if let Some(val) = self.core.get_from_top(|layer| layer.get(key).cloned()) {
            return Ok((self.core.state(), val.into()));
        }
        self.core.src.req(key)
    }
}

impl<Q, S, K, V> TakeSnapshot<Q> for Overridable<S, K, V>
where
    Q: ?Sized + Eq + Hash,
    S: TakeSnapshot<Q>,
    K: 'static + Clone + Eq + Hash + Borrow<Q>,
    V: 'static + Clone + Into<S::Output>,
{
    type SnapShot = Overriden<S::SnapShot, K, V>;
    type SnapShotErr = S::SnapShotErr;

    fn take_snapshot<'a, It>(&self, keys: It) -> Result<Self::SnapShot, Self::SnapShotErr>
    where
        It: IntoIterator<Item = &'a Q>,
        Q: 'a,
    {
        let items = keys.into_iter().collect::<Vec<_>>();
        let snapshot = self.core.src.take_snapshot(items.iter().map(|q| *q))?;

        let layer = items.iter().filter_map(|k| {
            self.core.get_from_bottom(|layer| {
                layer.get_key_value(k).map(|(k, v)| (k.clone(), v.clone()))
            })
        });

        Ok(Overriden::with_layer(
            self.core.desc(),
            snapshot,
            layer.collect(),
        ))
    }
}

impl<S, K, V> Overridable<S, K, V> {
    /// Push a new override layer.
    /// The state id of the node after the layer is pushed is returned.
    #[inline]
    pub fn push_layer(&mut self, layer: HashMap<K, V>) -> NodeStateId {
        self.core.push(layer)
    }

    /// Pop the top override layer.
    /// The state id of the node after the layer is popped is also returned.
    #[inline]
    pub fn pop_layer(&mut self) -> Option<(NodeStateId, HashMap<K, V>)> {
        self.core.pop()
    }

    /// Clear all override layers.
    /// The state id of the node after the layers are cleared is returned.
    #[inline]
    pub fn clear_layers(&mut self) -> NodeStateId {
        self.core.clear()
    }

    /// Num of override layers
    #[inline]
    pub fn num_layers(&self) -> usize {
        self.core.num_layers()
    }
}

// -----------------------------------------------------------------------------
// Overriden2Args
//
#[derive(Debug, Node)]
#[node(transparent = "core")]
pub struct Overriden2Args<S, K1, K2, V> {
    core: Arc<_UnaryPassThroughNode<S>>,
    layer: Arc<HashMap<K1, HashMap<K2, V>>>,
}

//
// construction
//
impl<S: Node, K1, K2, V> Overriden2Args<S, K1, K2, V>
where
    K1: Eq + Hash,
    K2: Eq + Hash,
{
    #[inline]
    fn with_layer(desc: impl Into<String>, src: S, layer: HashMap<K1, HashMap<K2, V>>) -> Self {
        Self {
            core: _UnaryPassThroughNode::new(src, desc),
            layer: Arc::new(layer),
        }
    }
}

impl<S, K1, K2, V> Clone for Overriden2Args<S, K1, K2, V> {
    #[inline]
    fn clone(&self) -> Self {
        Self {
            core: self.core.clone(),
            layer: self.layer.clone(),
        }
    }
}

//
// methods
//
impl<S, K1, K2, V> Overriden2Args<S, K1, K2, V> {
    #[inline]
    pub fn downstream(&self) -> &S {
        &self.core.src
    }
}

impl<Q1, Q2, S, K1, K2, V> DataSrc2Args<Q1, Q2> for Overriden2Args<S, K1, K2, V>
where
    Q1: ?Sized + Eq + Hash,
    Q2: ?Sized + Eq + Hash,
    S: DataSrc2Args<Q1, Q2>,
    K1: 'static + Eq + Hash + Borrow<Q1>,
    K2: 'static + Eq + Hash + Borrow<Q2>,
    V: 'static + Clone + Into<S::Output>,
{
    type Output = S::Output;
    type Err = S::Err;

    fn req(&self, key1: &Q1, key2: &Q2) -> Result<(super::NodeStateId, Self::Output), Self::Err> {
        if let Some(v) = self.layer.get(key1).and_then(|m| m.get(key2)) {
            return Ok((self.core.state(), v.clone().into()));
        }
        self.core.src.req(key1, key2)
    }
}

impl<Q1, Q2, S, K1, K2, V> TakeSnapshot2Args<Q1, Q2> for Overriden2Args<S, K1, K2, V>
where
    Q1: ?Sized + Eq + Hash,
    Q2: ?Sized + Eq + Hash,
    S: TakeSnapshot2Args<Q1, Q2>,
    K1: 'static + Eq + Hash + Borrow<Q1> + Clone,
    K2: 'static + Eq + Hash + Borrow<Q2> + Clone,
    V: 'static + Clone + Into<S::Output>,
{
    type SnapShot = Overriden2Args<S::SnapShot, K1, K2, V>;
    type SnapShotErr = S::SnapShotErr;

    fn take_snapshot<'a, It>(&self, keys: It) -> Result<Self::SnapShot, Self::SnapShotErr>
    where
        It: IntoIterator<Item = (&'a Q1, &'a Q2)>,
        Q1: 'a,
        Q2: 'a,
    {
        let items = keys.into_iter().collect::<Vec<_>>();
        let snapshot = self
            .core
            .src
            .take_snapshot(items.iter().map(|(q1, q2)| (*q1, *q2)))?;

        let contained = items.iter().filter_map(|(k1, k2)| {
            let fst = self.layer.get_key_value(k1);
            fst.and_then(|(k1, m)| m.get_key_value(k2).map(|(k2, v)| (k1, k2, v)))
        });
        let mut layer = HashMap::new();

        for (k1, k2, v) in contained {
            layer
                .entry(k1.clone())
                .or_insert_with(HashMap::new)
                .insert(k2.clone(), v.clone());
        }
        Ok(Overriden2Args::with_layer(
            self.core.desc(),
            snapshot,
            layer,
        ))
    }
}

// -----------------------------------------------------------------------------
// Overridable2Args
//
#[derive(Debug, Node)]
#[node(transparent = "core")]
pub struct Overridable2Args<S, K1, K2, V> {
    core: Arc<_node::_Node<S, HashMap<K1, HashMap<K2, V>>>>,
}

//
// construction
//
impl<S: Node, K1, K2, V> Overridable2Args<S, K1, K2, V>
where
    K1: 'static + Eq + Hash,
    K2: 'static + Eq + Hash,
    V: 'static,
{
    #[inline]
    fn _new(desc: impl Into<String>, src: S, layers: Vec<HashMap<K1, HashMap<K2, V>>>) -> Self {
        let core = _node::_Node::new(desc, src);
        core.extend(layers);
        Self { core }
    }
    #[inline]
    pub fn with_layer(desc: impl Into<String>, src: S, layer: HashMap<K1, HashMap<K2, V>>) -> Self {
        Self::_new(desc, src, vec![layer])
    }
    #[inline]
    pub fn new(desc: impl Into<String>, src: S) -> Self {
        Self::_new(desc, src, Vec::new())
    }
}

impl<S, K1, K2, V> Clone for Overridable2Args<S, K1, K2, V> {
    #[inline]
    fn clone(&self) -> Self {
        Self {
            core: self.core.clone(),
        }
    }
}

//
// methods
//
impl<S, K1, K2, V> Overridable2Args<S, K1, K2, V> {
    #[inline]
    pub fn downstream(&self) -> &S {
        &self.core.src
    }
}

impl<Q1, Q2, S, K1, K2, V> DataSrc2Args<Q1, Q2> for Overridable2Args<S, K1, K2, V>
where
    Q1: ?Sized + Eq + Hash,
    Q2: ?Sized + Eq + Hash,
    S: DataSrc2Args<Q1, Q2>,
    K1: 'static + Eq + Hash + Borrow<Q1>,
    K2: 'static + Eq + Hash + Borrow<Q2>,
    V: 'static + Clone + Into<S::Output>,
{
    type Output = S::Output;
    type Err = S::Err;

    fn req(&self, key1: &Q1, key2: &Q2) -> Result<(super::NodeStateId, Self::Output), Self::Err> {
        if let Some(v) = self
            .core
            .get_from_top(|layer| layer.get(key1).and_then(|m| m.get(key2).cloned()))
        {
            return Ok((self.core.state(), v.into()));
        }
        self.core.src.req(key1, key2)
    }
}

impl<Q1, Q2, S, K1, K2, V> TakeSnapshot2Args<Q1, Q2> for Overridable2Args<S, K1, K2, V>
where
    Q1: ?Sized + Eq + Hash,
    Q2: ?Sized + Eq + Hash,
    S: TakeSnapshot2Args<Q1, Q2>,
    K1: 'static + Eq + Hash + Borrow<Q1> + Clone,
    K2: 'static + Eq + Hash + Borrow<Q2> + Clone,
    V: 'static + Clone + Into<S::Output>,
{
    type SnapShot = Overriden2Args<S::SnapShot, K1, K2, V>;
    type SnapShotErr = S::SnapShotErr;

    fn take_snapshot<'a, It>(&self, keys: It) -> Result<Self::SnapShot, Self::SnapShotErr>
    where
        It: IntoIterator<Item = (&'a Q1, &'a Q2)>,
        Q1: 'a,
        Q2: 'a,
    {
        let items = keys.into_iter().collect::<Vec<_>>();
        let snapshot = self
            .core
            .src
            .take_snapshot(items.iter().map(|(q1, q2)| (*q1, *q2)))?;

        let mut layer = HashMap::new();

        let contained = items.iter().filter_map(|(k1, k2)| {
            self.core.get_from_bottom(|layer| {
                let fst = layer.get_key_value(k1);
                fst.and_then(|(k1, m)| {
                    let snd = m.get_key_value(k2);
                    snd.map(|(k2, v)| (k1.clone(), k2.clone(), v.clone()))
                })
            })
        });
        for (k1, k2, v) in contained {
            layer.entry(k1).or_insert_with(HashMap::new).insert(k2, v);
        }
        Ok(Overriden2Args::with_layer(
            self.core.desc(),
            snapshot,
            layer,
        ))
    }
}

impl<S, K1, K2, V> Overridable2Args<S, K1, K2, V>
where
    K1: Eq + Hash + Clone,
    K2: Eq + Hash + Clone,
{
    /// Push a new override layer.
    /// The state id of the node after the layer is pushed is returned.
    #[inline]
    pub fn push_layer(&mut self, layer: HashMap<K1, HashMap<K2, V>>) -> NodeStateId {
        self.core.push(layer)
    }

    /// Pop the top override layer.
    /// The state id of the node after the layer is popped is also returned.
    #[inline]
    pub fn pop_layer(&mut self) -> Option<(NodeStateId, HashMap<K1, HashMap<K2, V>>)> {
        self.core.pop()
    }

    /// Clear all override layers.
    #[inline]
    pub fn clear_layers(&mut self) -> NodeStateId {
        self.core.clear()
    }

    #[inline]
    pub fn num_layers(&self) -> usize {
        self.core.num_layers()
    }
}

// -----------------------------------------------------------------------------
// Overriden3Args
//
#[derive(Debug, Node)]
#[node(transparent = "core")]
pub struct Overriden3Args<S, K1, K2, K3, V> {
    core: Arc<_UnaryPassThroughNode<S>>,
    layer: Arc<HashMap<K1, HashMap<K2, HashMap<K3, V>>>>,
}

//
// construction
//
impl<S: Node, K1, K2, K3, V> Overriden3Args<S, K1, K2, K3, V>
where
    K1: Eq + Hash,
    K2: Eq + Hash,
    K3: Eq + Hash,
{
    #[inline]
    fn with_layer(
        desc: impl Into<String>,
        src: S,
        layer: HashMap<K1, HashMap<K2, HashMap<K3, V>>>,
    ) -> Self {
        Self {
            core: _UnaryPassThroughNode::new(src, desc),
            layer: Arc::new(layer),
        }
    }
}

impl<S, K1, K2, K3, V> Clone for Overriden3Args<S, K1, K2, K3, V> {
    #[inline]
    fn clone(&self) -> Self {
        Self {
            core: self.core.clone(),
            layer: self.layer.clone(),
        }
    }
}

//
// methods
//
impl<S, K1, K2, K3, V> Overriden3Args<S, K1, K2, K3, V> {
    #[inline]
    pub fn downstream(&self) -> &S {
        &self.core.src
    }
}

impl<Q1, Q2, Q3, S, K1, K2, K3, V> DataSrc3Args<Q1, Q2, Q3> for Overriden3Args<S, K1, K2, K3, V>
where
    Q1: ?Sized + Eq + Hash,
    Q2: ?Sized + Eq + Hash,
    Q3: ?Sized + Eq + Hash,
    S: DataSrc3Args<Q1, Q2, Q3>,
    K1: 'static + Eq + Hash + Borrow<Q1>,
    K2: 'static + Eq + Hash + Borrow<Q2>,
    K3: 'static + Eq + Hash + Borrow<Q3>,
    V: 'static + Clone + Into<S::Output>,
{
    type Output = S::Output;
    type Err = S::Err;

    fn req(
        &self,
        key1: &Q1,
        key2: &Q2,
        key3: &Q3,
    ) -> Result<(super::NodeStateId, Self::Output), Self::Err> {
        if let Some(v) = self
            .layer
            .get(key1)
            .and_then(|m1| m1.get(key2))
            .and_then(|m2| m2.get(key3))
        {
            return Ok((self.core.state(), v.clone().into()));
        }
        self.core.src.req(key1, key2, key3)
    }
}

impl<Q1, Q2, Q3, S, K1, K2, K3, V> TakeSnapshot3Args<Q1, Q2, Q3>
    for Overriden3Args<S, K1, K2, K3, V>
where
    Q1: ?Sized + Eq + Hash,
    Q2: ?Sized + Eq + Hash,
    Q3: ?Sized + Eq + Hash,
    S: TakeSnapshot3Args<Q1, Q2, Q3>,
    K1: 'static + Eq + Hash + Borrow<Q1> + Clone,
    K2: 'static + Eq + Hash + Borrow<Q2> + Clone,
    K3: 'static + Eq + Hash + Borrow<Q3> + Clone,
    V: 'static + Clone + Into<S::Output>,
{
    type SnapShot = Overriden3Args<S::SnapShot, K1, K2, K3, V>;
    type SnapShotErr = S::SnapShotErr;

    fn take_snapshot<'a, It>(&self, keys: It) -> Result<Self::SnapShot, Self::SnapShotErr>
    where
        It: IntoIterator<Item = (&'a Q1, &'a Q2, &'a Q3)>,
        Q1: 'a,
        Q2: 'a,
        Q3: 'a,
    {
        let items = keys.into_iter().collect::<Vec<_>>();
        let snapshot = self
            .core
            .src
            .take_snapshot(items.iter().map(|(q1, q2, q3)| (*q1, *q2, *q3)))?;
        let mut layer = HashMap::new();

        let contained = items.iter().filter_map(|(k1, k2, k3)| {
            let fst = self.layer.get_key_value(k1);
            fst.and_then(|(k1, m1)| {
                let snd = m1.get_key_value(k2);
                snd.and_then(|(k2, m2)| {
                    let thd = m2.get_key_value(k3);
                    thd.map(|(k3, v)| (k1.clone(), k2.clone(), k3.clone(), v.clone()))
                })
            })
        });
        for (k1, k2, k3, v) in contained {
            layer
                .entry(k1)
                .or_insert_with(HashMap::new)
                .entry(k2)
                .or_insert_with(HashMap::new)
                .insert(k3, v);
        }
        Ok(Overriden3Args::with_layer(
            self.core.desc(),
            snapshot,
            layer,
        ))
    }
}

// -----------------------------------------------------------------------------
// Overridable3Args
//
#[derive(Debug, Node)]
#[node(transparent = "core")]
pub struct Overridable3Args<S, K1, K2, K3, V> {
    core: Arc<_node::_Node<S, HashMap<K1, HashMap<K2, HashMap<K3, V>>>>>,
}

//
// construction
//
impl<S: Node, K1, K2, K3, V> Overridable3Args<S, K1, K2, K3, V>
where
    K1: 'static + Eq + Hash,
    K2: 'static + Eq + Hash,
    K3: 'static + Eq + Hash,
    V: 'static,
{
    #[inline]
    fn with_layer(
        desc: impl Into<String>,
        src: S,
        layers: Vec<HashMap<K1, HashMap<K2, HashMap<K3, V>>>>,
    ) -> Self {
        let core = _node::_Node::new(desc, src);
        core.extend(layers);
        Self { core }
    }
    #[inline]
    pub fn new(desc: impl Into<String>, src: S) -> Self {
        Self::with_layer(desc, src, Vec::new())
    }
}

impl<S, K1, K2, K3, V> Clone for Overridable3Args<S, K1, K2, K3, V> {
    #[inline]
    fn clone(&self) -> Self {
        Self {
            core: self.core.clone(),
        }
    }
}

//
// methods
//
impl<S, K1, K2, K3, V> Overridable3Args<S, K1, K2, K3, V> {
    #[inline]
    pub fn downstream(&self) -> &S {
        &self.core.src
    }
}

impl<Q1, Q2, Q3, S, K1, K2, K3, V> DataSrc3Args<Q1, Q2, Q3> for Overridable3Args<S, K1, K2, K3, V>
where
    Q1: ?Sized + Eq + Hash,
    Q2: ?Sized + Eq + Hash,
    Q3: ?Sized + Eq + Hash,
    S: DataSrc3Args<Q1, Q2, Q3>,
    K1: 'static + Eq + Hash + Borrow<Q1>,
    K2: 'static + Eq + Hash + Borrow<Q2>,
    K3: 'static + Eq + Hash + Borrow<Q3>,
    V: 'static + Clone + Into<S::Output>,
{
    type Output = S::Output;
    type Err = S::Err;

    fn req(
        &self,
        key1: &Q1,
        key2: &Q2,
        key3: &Q3,
    ) -> Result<(super::NodeStateId, Self::Output), Self::Err> {
        if let Some(v) = self.core.get_from_top(|layer| {
            let fst = layer.get(key1);
            let snd = fst.and_then(|m1| m1.get(key2));
            snd.and_then(|m2| m2.get(key3).cloned())
        }) {
            return Ok((self.core.state(), v.clone().into()));
        }
        self.core.src.req(key1, key2, key3)
    }
}

impl<Q1, Q2, Q3, S, K1, K2, K3, V> TakeSnapshot3Args<Q1, Q2, Q3>
    for Overridable3Args<S, K1, K2, K3, V>
where
    Q1: ?Sized + Eq + Hash,
    Q2: ?Sized + Eq + Hash,
    Q3: ?Sized + Eq + Hash,
    S: TakeSnapshot3Args<Q1, Q2, Q3>,
    K1: 'static + Eq + Hash + Borrow<Q1> + Clone,
    K2: 'static + Eq + Hash + Borrow<Q2> + Clone,
    K3: 'static + Eq + Hash + Borrow<Q3> + Clone,
    V: 'static + Clone + Into<S::Output>,
{
    type SnapShot = Overriden3Args<S::SnapShot, K1, K2, K3, V>;
    type SnapShotErr = S::SnapShotErr;

    fn take_snapshot<'a, It>(&self, keys: It) -> Result<Self::SnapShot, Self::SnapShotErr>
    where
        It: IntoIterator<Item = (&'a Q1, &'a Q2, &'a Q3)>,
        Q1: 'a,
        Q2: 'a,
        Q3: 'a,
    {
        let items = keys.into_iter().collect::<Vec<_>>();
        let snapshot = self
            .core
            .src
            .take_snapshot(items.iter().map(|(q1, q2, q3)| (*q1, *q2, *q3)))?;
        let mut layer = HashMap::new();

        let contained = items.iter().filter_map(|(k1, k2, k3)| {
            self.core.get_from_bottom(|layer| {
                let fst = layer.get_key_value(k1);
                fst.and_then(|(k1, m1)| {
                    let snd = m1.get_key_value(k2);
                    snd.and_then(|(k2, m2)| {
                        let trd = m2.get_key_value(k3);
                        trd.map(|(k3, v)| (k1.clone(), k2.clone(), k3.clone(), v.clone()))
                    })
                })
            })
        });
        for (k1, k2, k3, v) in contained {
            layer
                .entry(k1)
                .or_insert_with(HashMap::new)
                .entry(k2)
                .or_insert_with(HashMap::new)
                .insert(k3, v);
        }
        Ok(Overriden3Args::with_layer(
            self.core.desc(),
            snapshot,
            layer,
        ))
    }
}

impl<S, K1, K2, K3, V> Overridable3Args<S, K1, K2, K3, V>
where
    K1: Eq + Hash + Clone,
    K2: Eq + Hash + Clone,
    K3: Eq + Hash + Clone,
{
    /// Push a new override layer.
    #[inline]
    pub fn push_layer(&mut self, layer: HashMap<K1, HashMap<K2, HashMap<K3, V>>>) -> NodeStateId {
        self.core.push(layer)
    }

    /// Pop the top override layer.
    #[inline]
    pub fn pop_layer(&mut self) -> Option<(NodeStateId, HashMap<K1, HashMap<K2, HashMap<K3, V>>>)> {
        self.core.pop()
    }

    /// Pop the top override layer.
    #[inline]
    pub fn clear_layers(&mut self) -> NodeStateId {
        self.core.clear()
    }

    #[inline]
    pub fn num_layers(&self) -> usize {
        self.core.num_layers()
    }
}
