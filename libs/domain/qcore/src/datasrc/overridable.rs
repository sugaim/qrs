use std::{
    borrow::Borrow,
    collections::HashMap,
    hash::Hash,
    ops::Deref,
    sync::{Arc, Mutex},
};

use maplit::btreeset;
use qcore_derive::Listener;

use super::{
    DataSrc, Listener, Notifier, TakeSnapshot, Tree, _private::_UnaryPassThroughNode,
    node::DataSrc2Args, snapshot::TakeSnapshot3Args, DataSrc3Args, TakeSnapshot2Args,
};

// -----------------------------------------------------------------------------
// _Node
//
mod _node {
    use std::sync::{Arc, Mutex};

    use crate::datasrc::{Listener, NodeId, Notifier, PublisherState, StateId};

    #[derive(Debug)]
    pub(super) struct _Node<L> {
        src_id: NodeId,

        // as node state id, we use the combined value of the current override layer state
        // and the downstream state id.
        info: PublisherState,

        // state ids for override layers. To represent the layer structure,
        // we use stack(vec) to store the state ids.
        // the first element is for id when no override layer is applied.
        // the second element is for id when the first override layer is applied and so on.
        // hence, the last element is for the id when all override layers are applied
        // and the current state of the node.
        // if the top of override layer is popped, the state id is also popped.
        override_state: Vec<StateId>,

        // override layers
        layers: Vec<L>,
    }

    //
    // construction
    //
    impl<L: 'static + Send + Sync> _Node<L> {
        pub fn new_and_reg<S: Notifier>(
            desc: impl Into<String>,
            src: &mut S,
            layers: Vec<L>,
        ) -> Arc<Mutex<Self>> {
            let override_state = (0..(layers.len() + 1))
                .map(|_| StateId::gen())
                .collect::<Vec<_>>();
            let self_state = override_state.last().unwrap().clone();
            let res = Arc::new(Mutex::new(Self {
                src_id: src.id(),
                info: PublisherState::new(desc),
                override_state,
                layers,
            }));

            let lis = Arc::downgrade(&res);
            let state = src.accept_listener(lis) ^ self_state;
            res.lock().unwrap().info.set_state(state);
            res
        }
    }

    //
    // methods
    //
    impl<L> _Node<L> {
        /// Get the state id of the node.
        pub fn state(&self) -> StateId {
            self.info.state()
        }

        /// Desc
        pub fn desc(&self) -> &str {
            self.info.desc()
        }

        /// Get a value from the override layers.
        /// The value is found from the top layer to the bottom layer.
        pub fn get_from_top<O>(&self, f: impl Fn(&L) -> Option<O>) -> (StateId, Option<O>) {
            (self.state(), self.layers.iter().rev().find_map(f))
        }

        /// Get a value from the override layers.
        /// The value is found from the bottom layer to the top layer.
        pub fn get_from_bottom<O>(&self, f: impl Fn(&L) -> Option<O>) -> (StateId, Option<O>) {
            (self.state(), self.layers.iter().find_map(f))
        }

        /// Pop the top override layer.
        ///
        /// The state id of the node after the layer is popped is also returned.
        pub fn pop(&mut self) -> Option<(StateId, L)> {
            let popped = self.layers.pop();
            if popped.is_none() {
                return None;
            }
            let prev_state = self.override_state.pop().unwrap();
            let new_state = self.override_state.last().unwrap().clone();
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
        pub fn push(&mut self, layer: L) -> StateId {
            let prev_state = self.override_state.last().unwrap().clone();
            let new_state = StateId::gen();
            let mut node_state = self.info.state();

            // remove prev state(see bitxor property) and reflect new state
            node_state ^= prev_state;
            node_state ^= new_state;
            self.info.set_state(node_state);
            self.override_state.push(new_state);
            self.layers.push(layer);
            node_state
        }

        pub fn accept_listener(&mut self, subsc: std::sync::Weak<Mutex<dyn Listener>>) -> StateId {
            self.info.accept_listener(subsc)
        }

        pub fn remove_listener(&mut self, id: &NodeId) {
            self.info.remove_listener(id);
        }
    }

    impl<L> Extend<L> for _Node<L> {
        fn extend<T: IntoIterator<Item = L>>(&mut self, iter: T) {
            let orig_len = self.layers.len();
            self.layers.extend(iter);
            let num_incr = self.layers.len() - orig_len;

            let cur_state = self.override_state.last().unwrap().clone();
            self.override_state
                .extend((0..num_incr).map(|_| StateId::gen()));
            let new_state = self.override_state.last().unwrap().clone();

            let mut node_state = self.info.state();
            node_state ^= cur_state;
            node_state ^= new_state;
            self.info.set_state(node_state);
        }
    }

    impl<L: 'static + Send + Sync> Listener for _Node<L> {
        #[inline]
        fn id(&self) -> NodeId {
            self.info.id()
        }

        #[inline]
        fn listen(&mut self, publisher: &NodeId, state: &StateId) {
            if publisher != &self.src_id {
                return;
            }
            let layer_state = self.override_state.last().unwrap();
            self.info.set_state(state ^ layer_state);
        }
    }
}

// -----------------------------------------------------------------------------
// Overriden
//
#[derive(Debug, Listener)]
#[listener(transparent = "node")]
pub struct Overriden<S, K, V> {
    src: S,
    node: Arc<Mutex<_UnaryPassThroughNode>>,
    layer: HashMap<K, V>,
}

//
// construction
//
impl<S, K, V> Overriden<S, K, V>
where
    S: Notifier,
    K: 'static + Send + Sync,
    V: 'static + Send + Sync,
{
    #[inline]
    pub fn new(desc: impl Into<String>, mut src: S, layer: HashMap<K, V>) -> Self {
        let node = _UnaryPassThroughNode::new_and_reg(desc, &mut src);
        Self { src, node, layer }
    }
}

impl<S, K, V> Clone for Overriden<S, K, V>
where
    S: Clone + Notifier,
    K: 'static + Send + Sync + Clone,
    V: 'static + Send + Sync + Clone,
{
    #[inline]
    fn clone(&self) -> Self {
        Self::new(
            self.node.lock().unwrap().desc(),
            self.src.clone(),
            self.layer.clone(),
        )
    }
}

//
// methods
//
impl<S, K, V> Overriden<S, K, V> {
    #[inline]
    pub fn inner(&self) -> &S {
        &self.src
    }

    #[inline]
    pub fn inner_mut(&mut self) -> &mut S {
        &mut self.src
    }

    #[inline]
    pub fn into_inner(self) -> S {
        self.src
    }
}

impl<S, K, V> Notifier for Overriden<S, K, V>
where
    S: Notifier,
    K: 'static + Send + Sync,
    V: 'static + Send + Sync,
{
    #[inline]
    fn id(&self) -> super::NodeId {
        self.node.lock().unwrap().id()
    }

    #[inline]
    fn tree(&self) -> super::Tree {
        let (desc, id, state) = {
            let node = self.node.lock().unwrap();
            (node.desc().into(), node.id(), node.state())
        };
        Tree::Branch {
            desc,
            id,
            state,
            children: btreeset! {self.src.tree()},
        }
    }

    #[inline]
    fn accept_listener(&mut self, subsc: std::sync::Weak<Mutex<dyn Listener>>) -> super::StateId {
        self.node.lock().unwrap().accept_subscriber(subsc)
    }

    #[inline]
    fn remove_listener(&mut self, id: &super::NodeId) {
        self.node.lock().unwrap().remove_subscriber(id);
    }
}

impl<S, K, V> DataSrc for Overriden<S, K, V>
where
    S: DataSrc,
    S::Key: Eq + Hash,
    K: 'static + Send + Sync + Eq + Hash + Borrow<S::Key>,
    V: 'static + Send + Sync + Clone + Into<S::Output>,
{
    type Key = S::Key;
    type Output = S::Output;
    type Err = S::Err;

    #[inline]
    fn req(&self, key: &Self::Key) -> Result<(super::StateId, Self::Output), Self::Err> {
        let state = self.node.lock().unwrap().state();
        if let Some(val) = self.layer.get(key) {
            return Ok((state, val.clone().into()));
        }
        self.src.req(key).map(|(_, v)| (state, v))
    }
}

impl<S, K, V> TakeSnapshot for Overriden<S, K, V>
where
    S: TakeSnapshot,
    S::Key: Eq + Hash,
    K: 'static + Send + Sync + Eq + Hash + Clone + Borrow<S::Key>,
    V: 'static + Send + Sync + Clone + Into<S::Output>,
{
    type SnapShot = Overriden<S::SnapShot, K, V>;
    type SnapShotErr = S::SnapShotErr;

    fn take_snapshot<'a, It>(&self, keys: It) -> Result<Self::SnapShot, Self::SnapShotErr>
    where
        It: IntoIterator<Item = &'a Self::Key>,
        Self::Key: 'a,
    {
        let items = keys.into_iter().collect::<Vec<_>>();
        let snapshot = self.src.take_snapshot(items.iter().map(Deref::deref))?;
        let layer = items.iter().filter_map(|k| {
            self.layer
                .get_key_value(k)
                .map(|(k, v)| (k.clone(), v.clone()))
        });
        Ok(Overriden::new(
            self.node.lock().unwrap().desc(),
            snapshot,
            layer.collect(),
        ))
    }
}

// -----------------------------------------------------------------------------
// Overridable
//
#[derive(Debug, Listener)]
#[listener(transparent = "node")]
pub struct Overridable<S, K, V> {
    src: S,
    node: Arc<Mutex<_node::_Node<HashMap<K, V>>>>,
}

//
// construction
//
impl<S, K, V> Overridable<S, K, V>
where
    S: Notifier,
    K: 'static + Send + Sync,
    V: 'static + Send + Sync,
{
    #[inline]
    pub fn new(desc: impl Into<String>, mut src: S) -> Self {
        let node = _node::_Node::new_and_reg(desc, &mut src, Vec::new());
        Self { src, node }
    }

    #[inline]
    pub fn clone_without_override(&self) -> Self
    where
        S: Clone,
    {
        Self::new(self.node.lock().unwrap().desc(), self.src.clone())
    }
}

//
// methods
//
impl<S, K, V> Overridable<S, K, V> {
    #[inline]
    pub fn inner(&self) -> &S {
        &self.src
    }

    #[inline]
    pub fn inner_mut(&mut self) -> &mut S {
        &mut self.src
    }

    #[inline]
    pub fn into_inner(self) -> S {
        self.src
    }
}

impl<S, K, V> Notifier for Overridable<S, K, V>
where
    S: Notifier,
    K: 'static + Send + Sync,
    V: 'static + Send + Sync,
{
    #[inline]
    fn id(&self) -> super::NodeId {
        self.node.lock().unwrap().id()
    }

    #[inline]
    fn tree(&self) -> super::Tree {
        let (desc, id, state) = {
            let node = self.node.lock().unwrap();
            (node.desc().into(), node.id(), node.state())
        };
        Tree::Branch {
            desc,
            id,
            state,
            children: btreeset! {self.src.tree()},
        }
    }

    #[inline]
    fn accept_listener(&mut self, subsc: std::sync::Weak<Mutex<dyn Listener>>) -> super::StateId {
        self.node.lock().unwrap().accept_listener(subsc)
    }

    #[inline]
    fn remove_listener(&mut self, id: &super::NodeId) {
        self.node.lock().unwrap().remove_listener(id);
    }
}

impl<S, K, V> DataSrc for Overridable<S, K, V>
where
    S: DataSrc,
    S::Key: Eq + Hash,
    K: 'static + Send + Sync + Eq + Hash + Borrow<S::Key>,
    V: 'static + Send + Sync + Clone + Into<S::Output>,
{
    type Key = S::Key;
    type Output = S::Output;
    type Err = S::Err;

    #[inline]
    fn req(&self, key: &Self::Key) -> Result<(super::StateId, Self::Output), Self::Err> {
        let (state, val) = self
            .node
            .lock()
            .unwrap()
            .get_from_top(|layer| layer.get(key).cloned());
        if let Some(val) = val {
            return Ok((state, val.into()));
        }
        self.src.req(key).map(|(_, v)| (state, v))
    }
}

impl<S, K, V> TakeSnapshot for Overridable<S, K, V>
where
    S: TakeSnapshot,
    S::Key: Eq + Hash,
    K: 'static + Send + Sync + Eq + Hash + Clone + Borrow<S::Key>,
    V: 'static + Send + Sync + Clone + Into<S::Output>,
{
    type SnapShot = Overriden<S::SnapShot, K, V>;
    type SnapShotErr = S::SnapShotErr;

    fn take_snapshot<'a, It>(&self, keys: It) -> Result<Self::SnapShot, Self::SnapShotErr>
    where
        It: IntoIterator<Item = &'a Self::Key>,
        Self::Key: 'a,
    {
        let items = keys.into_iter().collect::<Vec<_>>();
        let snapshot = self.src.take_snapshot(items.iter().map(Deref::deref))?;
        let node = self.node.lock().unwrap();
        let layer = items.iter().filter_map(|k| {
            node.get_from_bottom(|layer| {
                layer.get_key_value(k).map(|(k, v)| (k.clone(), v.clone()))
            })
            .1
        });
        Ok(Overriden::new(
            self.node.lock().unwrap().desc(),
            snapshot,
            layer.collect(),
        ))
    }
}

impl<S, K, V> Overridable<S, K, V>
where
    S: DataSrc,
    S::Key: Eq + Hash,
    K: 'static + Send + Sync + Eq + Hash + Clone + Borrow<S::Key>,
    V: 'static + Send + Sync + Clone + Into<S::Output>,
{
    /// Temporarily override the layers.
    ///
    /// After the function is executed, the override layers are popped and the state of the node is restored.
    #[inline]
    pub fn temp_override<F, R>(&mut self, layer: HashMap<K, V>, f: F) -> R
    where
        F: FnOnce(&mut Self) -> R,
    {
        self.node.lock().unwrap().push(layer);
        let res = f(self);
        self.node.lock().unwrap().pop();
        res
    }

    #[inline]
    pub fn persistent_override(self, layer: HashMap<K, V>) -> Overriden<S, K, V> {
        Overriden::new(self.node.lock().unwrap().desc(), self.src, layer)
    }
}

// -----------------------------------------------------------------------------
// Overriden2Args
//
#[derive(Debug, Listener)]
#[listener(transparent = "node")]
pub struct Overriden2Args<S, K1, K2, V> {
    src: S,
    node: Arc<Mutex<_UnaryPassThroughNode>>,
    layer: HashMap<K1, HashMap<K2, V>>,
}

//
// construction
//
impl<S, K1, K2, V> Overriden2Args<S, K1, K2, V>
where
    S: Notifier,
    K1: 'static + Send + Sync,
    K2: 'static + Send + Sync,
    V: 'static + Send + Sync,
{
    #[inline]
    pub fn new(desc: impl Into<String>, mut src: S, layer: HashMap<K1, HashMap<K2, V>>) -> Self {
        let node = _UnaryPassThroughNode::new_and_reg(desc, &mut src);
        Self { src, node, layer }
    }
}

impl<S, K1, K2, V> Clone for Overriden2Args<S, K1, K2, V>
where
    S: Clone + Notifier,
    K1: 'static + Send + Sync + Clone,
    K2: 'static + Send + Sync + Clone,
    V: 'static + Send + Sync + Clone,
{
    #[inline]
    fn clone(&self) -> Self {
        Self::new(
            self.node.lock().unwrap().desc(),
            self.src.clone(),
            self.layer.clone(),
        )
    }
}

//
// methods
//
impl<S, K1, K2, V> Overriden2Args<S, K1, K2, V> {
    #[inline]
    pub fn inner(&self) -> &S {
        &self.src
    }

    #[inline]
    pub fn inner_mut(&mut self) -> &mut S {
        &mut self.src
    }

    #[inline]
    pub fn into_inner(self) -> S {
        self.src
    }
}

impl<S, K1, K2, V> Notifier for Overriden2Args<S, K1, K2, V>
where
    S: Notifier,
    K1: 'static + Send + Sync,
    K2: 'static + Send + Sync,
    V: 'static + Send + Sync,
{
    #[inline]
    fn id(&self) -> super::NodeId {
        self.node.lock().unwrap().id()
    }

    #[inline]
    fn tree(&self) -> super::Tree {
        let (desc, id, state) = {
            let node = self.node.lock().unwrap();
            (node.desc().into(), node.id(), node.state())
        };
        Tree::Branch {
            desc,
            id,
            state,
            children: btreeset! {self.src.tree()},
        }
    }

    #[inline]
    fn accept_listener(&mut self, subsc: std::sync::Weak<Mutex<dyn Listener>>) -> super::StateId {
        self.node.lock().unwrap().accept_subscriber(subsc)
    }

    #[inline]
    fn remove_listener(&mut self, id: &super::NodeId) {
        self.node.lock().unwrap().remove_subscriber(id);
    }
}

impl<S, K1, K2, V> DataSrc2Args for Overriden2Args<S, K1, K2, V>
where
    S: DataSrc2Args,
    S::Key1: Eq + Hash,
    S::Key2: Eq + Hash,
    K1: 'static + Send + Sync + Eq + Hash + Borrow<S::Key1>,
    K2: 'static + Send + Sync + Eq + Hash + Borrow<S::Key2>,
    V: 'static + Send + Sync + Clone + Into<S::Output>,
{
    type Key1 = S::Key1;
    type Key2 = S::Key2;
    type Output = S::Output;
    type Err = S::Err;

    #[inline]
    fn req(
        &self,
        key1: &S::Key1,
        key2: &S::Key2,
    ) -> Result<(super::StateId, Self::Output), Self::Err> {
        let state = self.node.lock().unwrap().state();
        if let Some(val) = self.layer.get(key1).and_then(|m| m.get(key2)) {
            return Ok((state, val.clone().into()));
        }
        self.src.req(key1, key2).map(|(_, v)| (state, v))
    }
}

impl<S, K1, K2, V> TakeSnapshot2Args for Overriden2Args<S, K1, K2, V>
where
    S: TakeSnapshot2Args,
    S::Key1: Eq + Hash,
    S::Key2: Eq + Hash,
    K1: 'static + Send + Sync + Eq + Hash + Clone + Borrow<S::Key1>,
    K2: 'static + Send + Sync + Eq + Hash + Clone + Borrow<S::Key2>,
    V: 'static + Send + Sync + Clone + Into<S::Output>,
{
    type SnapShot = Overriden2Args<S::SnapShot, K1, K2, V>;
    type SnapShotErr = S::SnapShotErr;

    fn take_snapshot<'a, It>(&self, keys: It) -> Result<Self::SnapShot, Self::SnapShotErr>
    where
        It: IntoIterator<Item = (&'a Self::Key1, &'a Self::Key2)>,
        Self::Key1: 'a,
        Self::Key2: 'a,
    {
        let items = keys.into_iter().collect::<Vec<_>>();
        let snapshot = self
            .src
            .take_snapshot(items.iter().map(|(k1, k2)| (*k1, *k2)))?;
        let contained = items.iter().filter_map(|(k1, k2)| {
            self.layer
                .get_key_value(k1)
                .and_then(|(k1, m)| m.get_key_value(k2).map(|(k2, v)| (k1, k2, v)))
        });
        let mut layer = HashMap::new();
        for (k1, k2, v) in contained {
            layer
                .entry(k1.clone())
                .or_insert_with(HashMap::new)
                .insert(k2.clone(), v.clone());
        }
        Ok(Overriden2Args::new(
            self.node.lock().unwrap().desc(),
            snapshot,
            layer,
        ))
    }
}

// -----------------------------------------------------------------------------
// Overridable2Args
//
#[derive(Debug, Listener)]
#[listener(transparent = "node")]
pub struct Overridable2Args<S, K1, K2, V> {
    src: S,
    node: Arc<Mutex<_node::_Node<HashMap<K1, HashMap<K2, V>>>>>,
}

//
// construction
//
impl<S, K1, K2, V> Overridable2Args<S, K1, K2, V>
where
    S: Notifier,
    K1: 'static + Send + Sync,
    K2: 'static + Send + Sync,
    V: 'static + Send + Sync,
{
    #[inline]
    pub fn new(desc: impl Into<String>, mut src: S) -> Self {
        let node = _node::_Node::new_and_reg(desc, &mut src, Vec::new());
        Self { src, node }
    }

    #[inline]
    pub fn clone_without_override(&self) -> Self
    where
        S: Clone,
    {
        Self::new(self.node.lock().unwrap().desc(), self.src.clone())
    }
}

//
// methods
//
impl<S, K1, K2, V> Overridable2Args<S, K1, K2, V> {
    #[inline]
    pub fn inner(&self) -> &S {
        &self.src
    }

    #[inline]
    pub fn inner_mut(&mut self) -> &mut S {
        &mut self.src
    }

    #[inline]
    pub fn into_inner(self) -> S {
        self.src
    }
}

impl<S, K1, K2, V> Notifier for Overridable2Args<S, K1, K2, V>
where
    S: Notifier,
    K1: 'static + Send + Sync,
    K2: 'static + Send + Sync,
    V: 'static + Send + Sync,
{
    #[inline]
    fn id(&self) -> super::NodeId {
        self.node.lock().unwrap().id()
    }

    #[inline]
    fn tree(&self) -> super::Tree {
        let (desc, id, state) = {
            let node = self.node.lock().unwrap();
            (node.desc().into(), node.id(), node.state())
        };
        Tree::Branch {
            desc,
            id,
            state,
            children: btreeset! {self.src.tree()},
        }
    }

    #[inline]
    fn accept_listener(&mut self, subsc: std::sync::Weak<Mutex<dyn Listener>>) -> super::StateId {
        self.node.lock().unwrap().accept_listener(subsc)
    }

    #[inline]
    fn remove_listener(&mut self, id: &super::NodeId) {
        self.node.lock().unwrap().remove_listener(id);
    }
}

impl<S, K1, K2, V> DataSrc2Args for Overridable2Args<S, K1, K2, V>
where
    S: DataSrc2Args,
    S::Key1: Eq + Hash,
    S::Key2: Eq + Hash,
    K1: 'static + Send + Sync + Eq + Hash + Borrow<S::Key1>,
    K2: 'static + Send + Sync + Eq + Hash + Borrow<S::Key2>,
    V: 'static + Send + Sync + Clone + Into<S::Output>,
{
    type Key1 = S::Key1;
    type Key2 = S::Key2;
    type Output = S::Output;
    type Err = S::Err;

    #[inline]
    fn req(
        &self,
        key1: &S::Key1,
        key2: &S::Key2,
    ) -> Result<(super::StateId, Self::Output), Self::Err> {
        let (state, val) = self
            .node
            .lock()
            .unwrap()
            .get_from_top(|layer| layer.get(key1).and_then(|m| m.get(key2).cloned()));
        if let Some(val) = val {
            return Ok((state, val.into()));
        }
        self.src.req(key1, key2).map(|(_, v)| (state, v))
    }
}

impl<S, K1, K2, V> TakeSnapshot2Args for Overridable2Args<S, K1, K2, V>
where
    S: TakeSnapshot2Args,
    S::Key1: Eq + Hash,
    S::Key2: Eq + Hash,
    K1: 'static + Send + Sync + Eq + Hash + Clone + Borrow<S::Key1>,
    K2: 'static + Send + Sync + Eq + Hash + Clone + Borrow<S::Key2>,
    V: 'static + Send + Sync + Clone + Into<S::Output>,
{
    type SnapShot = Overriden2Args<S::SnapShot, K1, K2, V>;
    type SnapShotErr = S::SnapShotErr;

    fn take_snapshot<'a, It>(&self, keys: It) -> Result<Self::SnapShot, Self::SnapShotErr>
    where
        It: IntoIterator<Item = (&'a Self::Key1, &'a Self::Key2)>,
        Self::Key1: 'a,
        Self::Key2: 'a,
    {
        let items = keys.into_iter().collect::<Vec<_>>();
        let snapshot = self
            .src
            .take_snapshot(items.iter().map(|(k1, k2)| (*k1, *k2)))?;
        let node = self.node.lock().unwrap();
        let contained = items.iter().filter_map(|(k1, k2)| {
            node.get_from_bottom(|layer| {
                layer.get_key_value(k1).and_then(|(k1, m)| {
                    m.get_key_value(k2)
                        .map(|(k2, v)| (k1.clone(), k2.clone(), v.clone()))
                })
            })
            .1
        });
        let mut layer = HashMap::new();
        for (k1, k2, v) in contained {
            layer.entry(k1).or_insert_with(HashMap::new).insert(k2, v);
        }
        Ok(Overriden2Args::new(
            self.node.lock().unwrap().desc(),
            snapshot,
            layer,
        ))
    }
}

impl<S, K1, K2, V> Overridable2Args<S, K1, K2, V>
where
    S: DataSrc2Args,
    S::Key1: Eq + Hash,
    S::Key2: Eq + Hash,
    K1: 'static + Send + Sync + Eq + Hash + Clone + Borrow<S::Key1>,
    K2: 'static + Send + Sync + Eq + Hash + Clone + Borrow<S::Key2>,
    V: 'static + Send + Sync + Clone + Into<S::Output>,
{
    /// Temporarily override the layers.
    ///
    /// After the function is executed, the override layers are popped and the state of the node is restored.
    #[inline]
    pub fn temp_override<F, R>(&mut self, layer: HashMap<K1, HashMap<K2, V>>, f: F) -> R
    where
        F: FnOnce(&mut Self) -> R,
    {
        self.node.lock().unwrap().push(layer);
        let res = f(self);
        self.node.lock().unwrap().pop();
        res
    }

    #[inline]
    pub fn persistent_override(
        self,
        layer: HashMap<K1, HashMap<K2, V>>,
    ) -> Overriden2Args<S, K1, K2, V> {
        Overriden2Args::new(self.node.lock().unwrap().desc(), self.src, layer)
    }
}

// -----------------------------------------------------------------------------
// Overriden3Args
//
#[derive(Debug, Listener)]
#[listener(transparent = "node")]
pub struct Overriden3Args<S, K1, K2, K3, V> {
    src: S,
    node: Arc<Mutex<_UnaryPassThroughNode>>,
    layer: HashMap<K1, HashMap<K2, HashMap<K3, V>>>,
}

//
// construction
//
impl<S, K1, K2, K3, V> Overriden3Args<S, K1, K2, K3, V>
where
    S: Notifier,
    K1: 'static + Send + Sync,
    K2: 'static + Send + Sync,
    K3: 'static + Send + Sync,
    V: 'static + Send + Sync,
{
    #[inline]
    pub fn new(
        desc: impl Into<String>,
        mut src: S,
        layer: HashMap<K1, HashMap<K2, HashMap<K3, V>>>,
    ) -> Self {
        let node = _UnaryPassThroughNode::new_and_reg(desc, &mut src);
        Self { src, node, layer }
    }
}

impl<S, K1, K2, K3, V> Clone for Overriden3Args<S, K1, K2, K3, V>
where
    S: Clone + Notifier,
    K1: 'static + Send + Sync + Clone,
    K2: 'static + Send + Sync + Clone,
    K3: 'static + Send + Sync + Clone,
    V: 'static + Send + Sync + Clone,
{
    #[inline]
    fn clone(&self) -> Self {
        Self::new(
            self.node.lock().unwrap().desc(),
            self.src.clone(),
            self.layer.clone(),
        )
    }
}

//
// methods
//
impl<S, K1, K2, K3, V> Overriden3Args<S, K1, K2, K3, V> {
    #[inline]
    pub fn inner(&self) -> &S {
        &self.src
    }

    #[inline]
    pub fn inner_mut(&mut self) -> &mut S {
        &mut self.src
    }

    #[inline]
    pub fn into_inner(self) -> S {
        self.src
    }
}

impl<S, K1, K2, K3, V> Notifier for Overriden3Args<S, K1, K2, K3, V>
where
    S: Notifier,
    K1: 'static + Send + Sync,
    K2: 'static + Send + Sync,
    K3: 'static + Send + Sync,
    V: 'static + Send + Sync,
{
    #[inline]
    fn id(&self) -> super::NodeId {
        self.node.lock().unwrap().id()
    }

    #[inline]
    fn tree(&self) -> super::Tree {
        let (desc, id, state) = {
            let node = self.node.lock().unwrap();
            (node.desc().into(), node.id(), node.state())
        };
        Tree::Branch {
            desc,
            id,
            state,
            children: btreeset! {self.src.tree()},
        }
    }

    #[inline]
    fn accept_listener(&mut self, subsc: std::sync::Weak<Mutex<dyn Listener>>) -> super::StateId {
        self.node.lock().unwrap().accept_subscriber(subsc)
    }

    #[inline]
    fn remove_listener(&mut self, id: &super::NodeId) {
        self.node.lock().unwrap().remove_subscriber(id);
    }
}

impl<S, K1, K2, K3, V> DataSrc3Args for Overriden3Args<S, K1, K2, K3, V>
where
    S: DataSrc3Args,
    S::Key1: Eq + Hash,
    S::Key2: Eq + Hash,
    S::Key3: Eq + Hash,
    K1: 'static + Send + Sync + Eq + Hash + Borrow<S::Key1>,
    K2: 'static + Send + Sync + Eq + Hash + Borrow<S::Key2>,
    K3: 'static + Send + Sync + Eq + Hash + Borrow<S::Key3>,
    V: 'static + Send + Sync + Clone + Into<S::Output>,
{
    type Key1 = S::Key1;
    type Key2 = S::Key2;
    type Key3 = S::Key3;
    type Output = S::Output;
    type Err = S::Err;

    #[inline]
    fn req(
        &self,
        key1: &S::Key1,
        key2: &S::Key2,
        key3: &S::Key3,
    ) -> Result<(super::StateId, Self::Output), Self::Err> {
        let state = self.node.lock().unwrap().state();
        if let Some(val) = self
            .layer
            .get(key1)
            .and_then(|m1| m1.get(key2).and_then(|m2| m2.get(key3).cloned()))
        {
            return Ok((state, val.clone().into()));
        }
        self.src.req(key1, key2, key3).map(|(_, v)| (state, v))
    }
}

impl<S, K1, K2, K3, V> TakeSnapshot3Args for Overriden3Args<S, K1, K2, K3, V>
where
    S: TakeSnapshot3Args,
    S::Key1: Eq + Hash,
    S::Key2: Eq + Hash,
    S::Key3: Eq + Hash,
    K1: 'static + Send + Sync + Eq + Hash + Clone + Borrow<S::Key1>,
    K2: 'static + Send + Sync + Eq + Hash + Clone + Borrow<S::Key2>,
    K3: 'static + Send + Sync + Eq + Hash + Clone + Borrow<S::Key3>,
    V: 'static + Send + Sync + Clone + Into<S::Output>,
{
    type SnapShot = Overriden3Args<S::SnapShot, K1, K2, K3, V>;
    type SnapShotErr = S::SnapShotErr;

    fn take_snapshot<'a, It>(&self, keys: It) -> Result<Self::SnapShot, Self::SnapShotErr>
    where
        It: IntoIterator<Item = (&'a Self::Key1, &'a Self::Key2, &'a Self::Key3)>,
        Self::Key1: 'a,
        Self::Key2: 'a,
        Self::Key3: 'a,
    {
        let items = keys.into_iter().collect::<Vec<_>>();
        let snapshot = self
            .src
            .take_snapshot(items.iter().map(|(k1, k2, k3)| (*k1, *k2, *k3)))?;
        let contained = items.iter().filter_map(|(k1, k2, k3)| {
            self.layer.get_key_value(k1).and_then(|(k1, m1)| {
                m1.get_key_value(k2)
                    .and_then(|(k2, m2)| m2.get_key_value(k3).map(|(k3, v)| (k1, k2, k3, v)))
            })
        });
        let mut layer = HashMap::new();
        for (k1, k2, k3, v) in contained {
            layer
                .entry(k1.clone())
                .or_insert_with(HashMap::new)
                .entry(k2.clone())
                .or_insert_with(HashMap::new)
                .insert(k3.clone(), v.clone());
        }
        Ok(Overriden3Args::new(
            self.node.lock().unwrap().desc(),
            snapshot,
            layer,
        ))
    }
}

// -----------------------------------------------------------------------------
// Overridable3Args
//

#[derive(Debug, Listener)]
#[listener(transparent = "node")]
pub struct Overridable3Args<S, K1, K2, K3, V> {
    src: S,
    node: Arc<Mutex<_node::_Node<HashMap<K1, HashMap<K2, HashMap<K3, V>>>>>>,
}

//
// construction
//
impl<S, K1, K2, K3, V> Overridable3Args<S, K1, K2, K3, V>
where
    S: Notifier,
    K1: 'static + Send + Sync,
    K2: 'static + Send + Sync,
    K3: 'static + Send + Sync,
    V: 'static + Send + Sync,
{
    #[inline]
    pub fn new(desc: impl Into<String>, mut src: S) -> Self {
        let node = _node::_Node::new_and_reg(desc, &mut src, Vec::new());
        Self { src, node }
    }

    #[inline]
    pub fn clone_without_override(&self) -> Self
    where
        S: Clone,
    {
        Self::new(self.node.lock().unwrap().desc(), self.src.clone())
    }
}

//
// methods
//
impl<S, K1, K2, K3, V> Overridable3Args<S, K1, K2, K3, V> {
    #[inline]
    pub fn inner(&self) -> &S {
        &self.src
    }

    #[inline]
    pub fn inner_mut(&mut self) -> &mut S {
        &mut self.src
    }

    #[inline]
    pub fn into_inner(self) -> S {
        self.src
    }
}

impl<S, K1, K2, K3, V> Notifier for Overridable3Args<S, K1, K2, K3, V>
where
    S: Notifier,
    K1: 'static + Send + Sync,
    K2: 'static + Send + Sync,
    K3: 'static + Send + Sync,
    V: 'static + Send + Sync,
{
    #[inline]
    fn id(&self) -> super::NodeId {
        self.node.lock().unwrap().id()
    }

    #[inline]
    fn tree(&self) -> super::Tree {
        let (desc, id, state) = {
            let node = self.node.lock().unwrap();
            (node.desc().into(), node.id(), node.state())
        };
        Tree::Branch {
            desc,
            id,
            state,
            children: btreeset! {self.src.tree()},
        }
    }

    #[inline]
    fn accept_listener(&mut self, subsc: std::sync::Weak<Mutex<dyn Listener>>) -> super::StateId {
        self.node.lock().unwrap().accept_listener(subsc)
    }

    #[inline]
    fn remove_listener(&mut self, id: &super::NodeId) {
        self.node.lock().unwrap().remove_listener(id);
    }
}

impl<S, K1, K2, K3, V> DataSrc3Args for Overridable3Args<S, K1, K2, K3, V>
where
    S: DataSrc3Args,
    S::Key1: Eq + Hash,
    S::Key2: Eq + Hash,
    S::Key3: Eq + Hash,
    K1: 'static + Send + Sync + Eq + Hash + Borrow<S::Key1>,
    K2: 'static + Send + Sync + Eq + Hash + Borrow<S::Key2>,
    K3: 'static + Send + Sync + Eq + Hash + Borrow<S::Key3>,
    V: 'static + Send + Sync + Clone + Into<S::Output>,
{
    type Key1 = S::Key1;
    type Key2 = S::Key2;
    type Key3 = S::Key3;
    type Output = S::Output;
    type Err = S::Err;

    #[inline]
    fn req(
        &self,
        key1: &S::Key1,
        key2: &S::Key2,
        key3: &S::Key3,
    ) -> Result<(super::StateId, Self::Output), Self::Err> {
        let (state, val) = self.node.lock().unwrap().get_from_top(|layer| {
            layer
                .get(key1)
                .and_then(|m1| m1.get(key2).and_then(|m2| m2.get(key3).cloned()))
        });
        if let Some(val) = val {
            return Ok((state, val.clone().into()));
        }
        self.src.req(key1, key2, key3).map(|(_, v)| (state, v))
    }
}

impl<S, K1, K2, K3, V> TakeSnapshot3Args for Overridable3Args<S, K1, K2, K3, V>
where
    S: TakeSnapshot3Args,
    S::Key1: Eq + Hash,
    S::Key2: Eq + Hash,
    S::Key3: Eq + Hash,
    K1: 'static + Send + Sync + Eq + Hash + Clone + Borrow<S::Key1>,
    K2: 'static + Send + Sync + Eq + Hash + Clone + Borrow<S::Key2>,
    K3: 'static + Send + Sync + Eq + Hash + Clone + Borrow<S::Key3>,
    V: 'static + Send + Sync + Clone + Into<S::Output>,
{
    type SnapShot = Overriden3Args<S::SnapShot, K1, K2, K3, V>;
    type SnapShotErr = S::SnapShotErr;

    fn take_snapshot<'a, It>(&self, keys: It) -> Result<Self::SnapShot, Self::SnapShotErr>
    where
        It: IntoIterator<Item = (&'a Self::Key1, &'a Self::Key2, &'a Self::Key3)>,
        Self::Key1: 'a,
        Self::Key2: 'a,
        Self::Key3: 'a,
    {
        let items = keys.into_iter().collect::<Vec<_>>();
        let snapshot = self
            .src
            .take_snapshot(items.iter().map(|(k1, k2, k3)| (*k1, *k2, *k3)))?;
        let node = self.node.lock().unwrap();
        let contained = items.iter().filter_map(|(k1, k2, k3)| {
            node.get_from_bottom(|layer| {
                layer.get_key_value(k1).and_then(|(k1, m1)| {
                    m1.get_key_value(k2).and_then(|(k2, m2)| {
                        m2.get_key_value(k3)
                            .map(|(k3, v)| (k1.clone(), k2.clone(), k3.clone(), v.clone()))
                    })
                })
            })
            .1
        });
        let mut layer = HashMap::new();
        for (k1, k2, k3, v) in contained {
            layer
                .entry(k1)
                .or_insert_with(HashMap::new)
                .entry(k2)
                .or_insert_with(HashMap::new)
                .insert(k3, v);
        }
        Ok(Overriden3Args::new(
            self.node.lock().unwrap().desc(),
            snapshot,
            layer,
        ))
    }
}

impl<S, K1, K2, K3, V> Overridable3Args<S, K1, K2, K3, V>
where
    S: DataSrc3Args,
    S::Key1: Eq + Hash,
    S::Key2: Eq + Hash,
    S::Key3: Eq + Hash,
    K1: 'static + Send + Sync + Eq + Hash + Clone + Borrow<S::Key1>,
    K2: 'static + Send + Sync + Eq + Hash + Clone + Borrow<S::Key2>,
    K3: 'static + Send + Sync + Eq + Hash + Clone + Borrow<S::Key3>,
    V: 'static + Send + Sync + Clone + Into<S::Output>,
{
    /// Temporarily override the layers.
    ///
    /// After the function is executed, the override layers are popped and the state of the node is restored.
    #[inline]
    pub fn temp_override<F, R>(
        &mut self,
        layer: HashMap<K1, HashMap<K2, HashMap<K3, V>>>,
        f: F,
    ) -> R
    where
        F: FnOnce(&mut Self) -> R,
    {
        self.node.lock().unwrap().push(layer);
        let res = f(self);
        self.node.lock().unwrap().pop();
        res
    }

    #[inline]
    pub fn persistent_override(
        self,
        layer: HashMap<K1, HashMap<K2, HashMap<K3, V>>>,
    ) -> Overriden3Args<S, K1, K2, K3, V> {
        Overriden3Args::new(self.node.lock().unwrap().desc(), self.src, layer)
    }
}
