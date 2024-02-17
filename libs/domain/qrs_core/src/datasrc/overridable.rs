use std::{
    borrow::Borrow,
    collections::HashMap,
    hash::Hash,
    ops::Deref,
    sync::{Arc, Mutex},
};

use maplit::btreeset;
use qrs_core_derive::{Listener, Node};

use super::{
    DataSrc, Listener, Notifier, TakeSnapshot, Tree, _private::_UnaryPassThroughNode,
    node::DataSrc2Args, snapshot::TakeSnapshot3Args, DataSrc3Args, Node, TakeSnapshot2Args,
};

// -----------------------------------------------------------------------------
// _Node
//
mod _node {
    use std::sync::{Arc, Mutex};

    use crate::datasrc::{Listener, Node, NodeId, Notifier, PublisherState, StateId};

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
            let self_state = *override_state.last().unwrap();
            let res = Arc::new(Mutex::new(Self {
                src_id: src.id(),
                info: PublisherState::new(desc),
                override_state,
                layers,
            }));

            src.accept_listener(Arc::downgrade(&res) as _);
            let state = src.state() ^ self_state;
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
        pub fn get_from_top<O>(&self, f: impl Fn(&L) -> Option<O>) -> Option<O> {
            self.layers.iter().rev().find_map(f)
        }

        /// Get a value from the override layers.
        /// The value is found from the bottom layer to the top layer.
        pub fn get_from_bottom<O>(&self, f: impl Fn(&L) -> Option<O>) -> Option<O> {
            self.layers.iter().find_map(f)
        }

        /// Pop the top override layer.
        ///
        /// The state id of the node after the layer is popped is also returned.
        pub fn pop(&mut self) -> Option<(StateId, L)> {
            let popped = self.layers.pop();
            popped.as_ref()?;
            let prev_state = self.override_state.pop().unwrap();
            let new_state = *self.override_state.last().unwrap();
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
            let prev_state = *self.override_state.last().unwrap();
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

            let cur_state = *self.override_state.last().unwrap();
            self.override_state
                .extend((0..num_incr).map(|_| StateId::gen()));
            let new_state = *self.override_state.last().unwrap();

            let mut node_state = self.info.state();
            node_state ^= cur_state;
            node_state ^= new_state;
            self.info.set_state(node_state);
        }
    }

    impl<L: 'static + Send + Sync> Node for _Node<L> {
        #[inline]
        fn id(&self) -> NodeId {
            self.info.id()
        }
    }

    impl<L: 'static + Send + Sync> Listener for _Node<L> {
        #[inline]
        fn listen(&mut self, publisher: &NodeId, state: &StateId) {
            if publisher != &self.src_id {
                return;
            }
            let layer_state = self.override_state.last().unwrap();
            self.info.set_state(state ^ layer_state);
        }
    }

    pub(super) type SharedNode<L> = Arc<Mutex<_Node<L>>>;
}

// -----------------------------------------------------------------------------
// Overriden
//
#[derive(Debug, Node, Listener)]
#[node(transparent = "node")]
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
    fn state(&self) -> super::StateId {
        self.node.lock().unwrap().state()
    }

    #[inline]
    fn tree(&self) -> super::Tree {
        let (desc, id, state) = {
            let node = self.node.lock().unwrap();
            (node.desc(), node.id(), node.state())
        };
        Tree::Branch {
            desc,
            id,
            state,
            children: btreeset! {self.src.tree()},
        }
    }

    #[inline]
    fn accept_listener(&mut self, subsc: std::sync::Weak<Mutex<dyn Listener>>) {
        self.node.lock().unwrap().accept_subscriber(subsc);
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
    fn req(&self, key: &Self::Key) -> Result<Self::Output, Self::Err> {
        self.layer
            .get(key)
            .map(|v| Ok(v.clone().into()))
            .unwrap_or_else(|| self.src.req(key))
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
#[derive(Debug, Node, Listener)]
#[node(transparent = "node")]
#[listener(transparent = "node")]
pub struct Overridable<S, K, V> {
    src: S,
    node: _node::SharedNode<HashMap<K, V>>,
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
    fn state(&self) -> super::StateId {
        self.node.lock().unwrap().state()
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
    fn accept_listener(&mut self, subsc: std::sync::Weak<Mutex<dyn Listener>>) {
        self.node.lock().unwrap().accept_listener(subsc);
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
    fn req(&self, key: &Self::Key) -> Result<Self::Output, Self::Err> {
        self.node
            .lock()
            .unwrap()
            .get_from_top(|layer| layer.get(key).cloned())
            .map(|v| Ok(v.into()))
            .unwrap_or_else(|| self.src.req(key))
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
    pub fn persistent_override(mut self, layer: HashMap<K, V>) -> Overriden<S, K, V> {
        let (id, desc) = {
            let node = self.node.lock().unwrap();
            (node.id(), node.desc().to_owned())
        };
        self.src.remove_listener(&id);
        Overriden::new(desc, self.src, layer)
    }
}

// -----------------------------------------------------------------------------
// Overriden2Args
//
#[derive(Debug, Node, Listener)]
#[node(transparent = "node")]
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
    fn state(&self) -> super::StateId {
        self.node.lock().unwrap().state()
    }

    #[inline]
    fn tree(&self) -> super::Tree {
        let (desc, id, state) = {
            let node = self.node.lock().unwrap();
            (node.desc(), node.id(), node.state())
        };
        Tree::Branch {
            desc,
            id,
            state,
            children: btreeset! {self.src.tree()},
        }
    }

    #[inline]
    fn accept_listener(&mut self, subsc: std::sync::Weak<Mutex<dyn Listener>>) {
        self.node.lock().unwrap().accept_subscriber(subsc);
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
    fn req(&self, key1: &S::Key1, key2: &S::Key2) -> Result<Self::Output, Self::Err> {
        self.layer
            .get(key1)
            .and_then(|m| m.get(key2))
            .map(|v| Ok(v.clone().into()))
            .unwrap_or_else(|| self.src.req(key1, key2))
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
#[derive(Debug, Node, Listener)]
#[node(transparent = "node")]
#[listener(transparent = "node")]
pub struct Overridable2Args<S, K1, K2, V> {
    src: S,
    node: _node::SharedNode<HashMap<K1, HashMap<K2, V>>>,
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
    fn state(&self) -> super::StateId {
        self.node.lock().unwrap().state()
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
    fn accept_listener(&mut self, subsc: std::sync::Weak<Mutex<dyn Listener>>) {
        self.node.lock().unwrap().accept_listener(subsc);
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
    fn req(&self, key1: &S::Key1, key2: &S::Key2) -> Result<Self::Output, Self::Err> {
        self.node
            .lock()
            .unwrap()
            .get_from_top(|layer| layer.get(key1).and_then(|m| m.get(key2).cloned()))
            .map(|v| Ok(v.into()))
            .unwrap_or_else(|| self.src.req(key1, key2))
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
        mut self,
        layer: HashMap<K1, HashMap<K2, V>>,
    ) -> Overriden2Args<S, K1, K2, V> {
        let (id, desc) = {
            let node = self.node.lock().unwrap();
            (node.id(), node.desc().to_owned())
        };
        self.src.remove_listener(&id);
        Overriden2Args::new(desc, self.src, layer)
    }
}

// -----------------------------------------------------------------------------
// Overriden3Args
//
#[derive(Debug, Node, Listener)]
#[node(transparent = "node")]
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
    fn state(&self) -> super::StateId {
        self.node.lock().unwrap().state()
    }

    #[inline]
    fn tree(&self) -> super::Tree {
        let (desc, id, state) = {
            let node = self.node.lock().unwrap();
            (node.desc(), node.id(), node.state())
        };
        Tree::Branch {
            desc,
            id,
            state,
            children: btreeset! {self.src.tree()},
        }
    }

    #[inline]
    fn accept_listener(&mut self, subsc: std::sync::Weak<Mutex<dyn Listener>>) {
        self.node.lock().unwrap().accept_subscriber(subsc);
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
    ) -> Result<Self::Output, Self::Err> {
        self.layer
            .get(key1)
            .and_then(|m1| m1.get(key2).and_then(|m2| m2.get(key3)))
            .map(|v| Ok(v.clone().into()))
            .unwrap_or_else(|| self.src.req(key1, key2, key3))
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

#[derive(Debug, Node, Listener)]
#[node(transparent = "node")]
#[listener(transparent = "node")]
#[allow(clippy::type_complexity)]
pub struct Overridable3Args<S, K1, K2, K3, V> {
    src: S,
    node: _node::SharedNode<HashMap<K1, HashMap<K2, HashMap<K3, V>>>>,
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
    fn state(&self) -> super::StateId {
        self.node.lock().unwrap().state()
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
    fn accept_listener(&mut self, subsc: std::sync::Weak<Mutex<dyn Listener>>) {
        self.node.lock().unwrap().accept_listener(subsc);
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
    ) -> Result<Self::Output, Self::Err> {
        self.node
            .lock()
            .unwrap()
            .get_from_top(|layer| {
                layer
                    .get(key1)
                    .and_then(|m1| m1.get(key2).and_then(|m2| m2.get(key3).cloned()))
            })
            .map(|v| Ok(v.into()))
            .unwrap_or_else(|| self.src.req(key1, key2, key3))
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
        mut self,
        layer: HashMap<K1, HashMap<K2, HashMap<K3, V>>>,
    ) -> Overriden3Args<S, K1, K2, K3, V> {
        let (id, desc) = {
            let node = self.node.lock().unwrap();
            (node.id(), node.desc().to_owned())
        };
        self.src.remove_listener(&id);
        Overriden3Args::new(desc, self.src, layer)
    }
}

// =============================================================================
#[cfg(test)]
mod tests {
    use std::sync::{Arc, Mutex};

    use maplit::hashmap;
    use rstest::{fixture, rstest};

    use crate::datasrc::{
        node::DataSrc2Args, DataSrc, DataSrc3Args, Node, Notifier, OnMemorySrc, OnMemorySrc2Args,
        OnMemorySrc3Args, Tree,
    };

    #[fixture]
    fn src_1arg() -> OnMemorySrc<u64, u64> {
        OnMemorySrc::with_data("map", hashmap! {1 => 10, 2 => 20, 3 => 30})
    }

    #[fixture]
    fn src_2args() -> OnMemorySrc2Args<u64, u64, u64> {
        OnMemorySrc2Args::with_data(
            "map",
            hashmap! {
                1 => hashmap!{10 => 100, 20 => 200, 30 => 300},
                2 => hashmap!{10 => 1000, 20 => 2000, 30 => 3000},
                3 => hashmap!{10 => 10000, 20 => 20000, 30 => 30000},
            },
        )
    }

    #[fixture]
    fn src_3args() -> OnMemorySrc3Args<u64, u64, u64, u64> {
        OnMemorySrc3Args::with_data(
            "map",
            hashmap! {
                1 => hashmap!{10 => hashmap!{100 => 1000, 200 => 2000, 300 => 3000}, 20 => hashmap!{100 => 10000, 200 => 20000, 300 => 30000}, 30 => hashmap!{100 => 100000, 200 => 200000, 300 => 300000}},
                2 => hashmap!{10 => hashmap!{100 => 1000000, 200 => 2000000, 300 => 3000000}, 20 => hashmap!{100 => 10000000, 200 => 20000000, 300 => 30000000}, 30 => hashmap!{100 => 100000000, 200 => 200000000, 300 => 300000000}},
                3 => hashmap!{10 => hashmap!{100 => 1000000000, 200 => 2000000000, 300 => 3000000000}, 20 => hashmap!{100 => 10000000000, 200 => 20000000000, 300 => 30000000000}, 30 => hashmap!{100 => 100000000000, 200 => 200000000000, 300 => 300000000000}},
            },
        )
    }

    #[rstest]
    fn test_overridable_1arg(src_1arg: OnMemorySrc<u64, u64>) {
        let mut src = Arc::new(Mutex::new(src_1arg)).overridable::<u64, u64>("overridable");

        let state = src.state();
        assert_eq!(src.req(&1).unwrap(), 10);
        assert_eq!(state, src.state());
        assert_eq!(src.req(&2).unwrap(), 20);
        assert_eq!(state, src.state());
        assert_eq!(src.req(&3).unwrap(), 30);
        assert_eq!(state, src.state());
        assert!(src.req(&4).is_err());
        assert_eq!(state, src.state());
        assert!(src.req(&5).is_err());
        assert_eq!(state, src.state());

        let res = src.temp_override(hashmap! {1 => 11, 4 => 4}, |src| {
            let new_state = src.state();
            assert_ne!(state, new_state);
            assert_eq!(src.req(&1).unwrap(), 11);
            assert_eq!(new_state, src.state());
            assert_eq!(src.req(&2).unwrap(), 20);
            assert_eq!(new_state, src.state());
            assert_eq!(src.req(&3).unwrap(), 30);
            assert_eq!(new_state, src.state());
            assert_eq!(src.req(&4).unwrap(), 4);
            assert_eq!(new_state, src.state());
            assert!(src.req(&5).is_err());
            assert_eq!(new_state, src.state());

            let res = src.temp_override(hashmap! {1 => 42}, |src| {
                let new_state2 = src.state();
                assert_ne!(new_state, new_state2);
                assert_eq!(src.req(&1).unwrap(), 42);
                assert_eq!(new_state2, src.state());
                assert_eq!(src.req(&2).unwrap(), 20);
                assert_eq!(new_state2, src.state());
                assert_eq!(src.req(&3).unwrap(), 30);
                assert_eq!(new_state2, src.state());
                assert_eq!(src.req(&4).unwrap(), 4);
                assert_eq!(new_state2, src.state());
                assert!(src.req(&5).is_err());
                assert_eq!(new_state2, src.state());

                src.req(&1).unwrap()
            });
            assert_eq!(src.req(&1).unwrap(), 11);
            assert_eq!(new_state, src.state());

            res
        });
        assert_eq!(res, 42);

        assert_eq!(src.req(&1).unwrap(), 10);
        assert_eq!(state, src.state());
        assert_eq!(src.req(&2).unwrap(), 20);
        assert_eq!(state, src.state());
        assert_eq!(src.req(&3).unwrap(), 30);
        assert_eq!(state, src.state());
        assert!(src.req(&4).is_err());
        assert_eq!(state, src.state());
        assert!(src.req(&5).is_err());
        assert_eq!(state, src.state());
    }

    #[rstest]
    fn test_overridable_1arg_clone(src_1arg: OnMemorySrc<u64, u64>) {
        let mut src = src_1arg.overridable::<u64, u64>("overridable");
        let src2 = src.clone_without_override();

        assert_ne!(src.id(), src2.id());

        assert_eq!(src.req(&1).unwrap(), 10);
        assert_eq!(src2.req(&1).unwrap(), 10);
        assert_eq!(src.req(&2).unwrap(), 20);
        assert_eq!(src2.req(&2).unwrap(), 20);

        src.temp_override(hashmap! {1 => 1, 2 => 2}, |src| {
            assert_eq!(src.req(&1).unwrap(), 1);
            assert_eq!(src.req(&2).unwrap(), 2);
            assert_eq!(src2.req(&1).unwrap(), 10);
            assert_eq!(src2.req(&2).unwrap(), 20);
        });
    }

    #[rstest]
    fn test_overridable_1arg_state_change(src_1arg: OnMemorySrc<u64, u64>) {
        let mut src = src_1arg.overridable::<u64, u64>("overridable");

        let state = src.state();
        assert_eq!(src.req(&1).unwrap(), 10);
        assert_eq!(state, src.state());

        src.inner_mut().insert(1, 11);
        assert_eq!(src.req(&1).unwrap(), 11);
        assert_ne!(state, src.state());
    }

    #[rstest]
    fn test_overridable_1arg_tree(src_1arg: OnMemorySrc<u64, u64>) {
        let src = src_1arg.overridable::<u64, u64>("overridable");
        let Tree::Branch {
            desc,
            id,
            state,
            children,
        } = src.tree()
        else {
            panic!()
        };
        assert_eq!(desc, "overridable");
        assert_eq!(id, src.id());
        assert_eq!(state, src.state());
        assert_eq!(children.len(), 1);
        assert_eq!(children.iter().next().unwrap(), &src.inner().tree());
    }

    #[rstest]
    fn test_overridable_1arg_persistent_override(src_1arg: OnMemorySrc<u64, u64>) {
        let src = src_1arg.overridable::<u64, u64>("overridable");
        let overriden = src.persistent_override(hashmap! {1 => 11, 4 => 4});

        assert_eq!(overriden.req(&1).unwrap(), 11);
        assert_eq!(overriden.req(&2).unwrap(), 20);
        assert_eq!(overriden.req(&3).unwrap(), 30);
        assert_eq!(overriden.req(&4).unwrap(), 4);
        assert!(overriden.req(&5).is_err());
    }

    #[rstest]
    fn test_overridable_2args(src_2args: OnMemorySrc2Args<u64, u64, u64>) {
        let mut src = Arc::new(Mutex::new(src_2args)).overridable::<u64, u64, u64>("overridable");

        let state = src.state();
        assert_eq!(src.req(&1, &10).unwrap(), 100);
        assert_eq!(state, src.state());
        assert_eq!(src.req(&2, &20).unwrap(), 2000);
        assert_eq!(state, src.state());
        assert_eq!(src.req(&3, &30).unwrap(), 30000);
        assert_eq!(state, src.state());
        assert!(src.req(&4, &40).is_err());
        assert_eq!(state, src.state());
        assert!(src.req(&5, &50).is_err());
        assert_eq!(state, src.state());

        let res = src.temp_override(
            hashmap! {1 => hashmap!{10 => 101, 40 => 4}, 4 => hashmap!{40 => 44}},
            |src| {
                let new_state = src.state();
                assert_ne!(state, new_state);
                assert_eq!(src.req(&1, &10).unwrap(), 101);
                assert_eq!(new_state, src.state());
                assert_eq!(src.req(&2, &20).unwrap(), 2000);
                assert_eq!(new_state, src.state());
                assert_eq!(src.req(&3, &30).unwrap(), 30000);
                assert_eq!(new_state, src.state());
                assert_eq!(src.req(&4, &40).unwrap(), 44);
                assert_eq!(new_state, src.state());
                assert!(src.req(&5, &50).is_err());
                assert_eq!(new_state, src.state());

                let res = src.temp_override(
                    hashmap! {1 => hashmap!{10 => 42}, 4 => hashmap!{40 => 44}},
                    |src| {
                        let new_state2 = src.state();
                        assert_ne!(new_state, new_state2);
                        assert_eq!(src.req(&1, &10).unwrap(), 42);
                        assert_eq!(new_state2, src.state());
                        assert_eq!(src.req(&2, &20).unwrap(), 2000);
                        assert_eq!(new_state2, src.state());
                        assert_eq!(src.req(&3, &30).unwrap(), 30000);
                        assert_eq!(new_state2, src.state());
                        assert_eq!(src.req(&4, &40).unwrap(), 44);
                        assert_eq!(new_state2, src.state());
                        assert!(src.req(&5, &50).is_err());
                        assert_eq!(new_state2, src.state());

                        src.req(&1, &10).unwrap()
                    },
                );
                assert_eq!(src.req(&1, &10).unwrap(), 101);
                assert_eq!(new_state, src.state());

                res
            },
        );
        assert_eq!(res, 42);

        assert_eq!(src.req(&1, &10).unwrap(), 100);
        assert_eq!(state, src.state());
        assert_eq!(src.req(&2, &20).unwrap(), 2000);
        assert_eq!(state, src.state());
        assert_eq!(src.req(&3, &30).unwrap(), 30000);
        assert_eq!(state, src.state());
        assert!(src.req(&4, &40).is_err());
        assert_eq!(state, src.state());
        assert!(src.req(&5, &50).is_err());
        assert_eq!(state, src.state());
    }

    #[rstest]
    fn test_overridable_2args_clone(src_2args: OnMemorySrc2Args<u64, u64, u64>) {
        let mut src = src_2args.overridable::<u64, u64, u64>("overridable");
        let src2 = src.clone_without_override();

        assert_ne!(src.id(), src2.id());

        assert_eq!(src.req(&1, &10).unwrap(), 100);
        assert_eq!(src2.req(&1, &10).unwrap(), 100);
        assert_eq!(src.req(&2, &20).unwrap(), 2000);
        assert_eq!(src2.req(&2, &20).unwrap(), 2000);

        src.temp_override(
            hashmap! {1 => hashmap!{10 => 1, 20 => 2}, 2 => hashmap!{20 => 20}},
            |src| {
                assert_eq!(src.req(&1, &10).unwrap(), 1);
                assert_eq!(src.req(&2, &20).unwrap(), 20);
                assert_eq!(src2.req(&1, &10).unwrap(), 100);
                assert_eq!(src2.req(&2, &20).unwrap(), 2000);
            },
        );
    }

    #[rstest]
    fn test_overridable_2args_state_change(src_2args: OnMemorySrc2Args<u64, u64, u64>) {
        let mut src = src_2args.overridable::<u64, u64, u64>("overridable");

        let state = src.state();
        assert_eq!(src.req(&1, &10).unwrap(), 100);
        assert_eq!(state, src.state());

        src.inner_mut().insert(1, 10, 101);
        assert_eq!(src.req(&1, &10).unwrap(), 101);
        assert_ne!(state, src.state());
    }

    #[rstest]
    fn test_overridable_2args_tree(src_2args: OnMemorySrc2Args<u64, u64, u64>) {
        let src = src_2args.overridable::<u64, u64, u64>("overridable");
        let Tree::Branch {
            desc,
            id,
            state,
            children,
        } = src.tree()
        else {
            panic!()
        };
        assert_eq!(desc, "overridable");
        assert_eq!(id, src.id());
        assert_eq!(state, src.state());
        assert_eq!(children.len(), 1);
        assert_eq!(children.iter().next().unwrap(), &src.inner().tree());
    }

    #[rstest]
    fn test_overridable_2args_persistent_override(src_2args: OnMemorySrc2Args<u64, u64, u64>) {
        let src = src_2args.overridable::<u64, u64, u64>("overridable");
        let overriden = src.persistent_override(
            hashmap! {1 => hashmap!{10 => 101, 40 => 4}, 4 => hashmap!{40 => 44}},
        );

        assert_eq!(overriden.req(&1, &10).unwrap(), 101);
        assert_eq!(overriden.req(&2, &20).unwrap(), 2000);
        assert_eq!(overriden.req(&3, &30).unwrap(), 30000);
        assert_eq!(overriden.req(&4, &40).unwrap(), 44);
        assert!(overriden.req(&5, &50).is_err());
    }

    #[rstest]
    fn test_overridable_3args(src_3args: OnMemorySrc3Args<u64, u64, u64, u64>) {
        let mut src =
            Arc::new(Mutex::new(src_3args)).overridable::<u64, u64, u64, u64>("overridable");

        let state = src.state();
        assert_eq!(src.req(&1, &10, &100).unwrap(), 1000);
        assert_eq!(state, src.state());
        assert_eq!(src.req(&2, &20, &200).unwrap(), 20000000);
        assert_eq!(state, src.state());
        assert_eq!(src.req(&3, &30, &300).unwrap(), 300000000000);
        assert_eq!(state, src.state());
        assert!(src.req(&4, &40, &400).is_err());
        assert_eq!(state, src.state());
        assert!(src.req(&5, &50, &500).is_err());
        assert_eq!(state, src.state());

        let res = src.temp_override(
            hashmap! {
                1 => hashmap!{10 => hashmap!{100 => 1001, 400 => 4}, 4 => hashmap!{400 => 44}},
                4 => hashmap!{40 => hashmap!{400 => 444}},
            },
            |src| {
                let new_state = src.state();
                assert_ne!(state, new_state);
                assert_eq!(src.req(&1, &10, &100).unwrap(), 1001);
                assert_eq!(new_state, src.state());
                assert_eq!(src.req(&1, &10, &400).unwrap(), 4);
                assert_eq!(new_state, src.state());
                assert_eq!(src.req(&1, &4, &400).unwrap(), 44);
                assert_eq!(src.req(&2, &20, &200).unwrap(), 20000000);
                assert_eq!(new_state, src.state());
                assert_eq!(src.req(&3, &30, &300).unwrap(), 300000000000);
                assert_eq!(new_state, src.state());
                assert_eq!(src.req(&4, &40, &400).unwrap(), 444);
                assert_eq!(new_state, src.state());
                assert!(src.req(&5, &50, &500).is_err());
                assert_eq!(new_state, src.state());

                let res = src.temp_override(
                    hashmap! {
                        1 => hashmap!{10 => hashmap!{100 => 42}, 4 => hashmap!{400 => 44}},
                        4 => hashmap!{40 => hashmap!{400 => 444}},
                    },
                    |src| {
                        let new_state2 = src.state();
                        assert_ne!(new_state, new_state2);
                        assert_eq!(src.req(&1, &10, &100).unwrap(), 42);
                        assert_eq!(new_state2, src.state());
                        assert_eq!(src.req(&1, &10, &400).unwrap(), 4);
                        assert_eq!(src.req(&2, &20, &200).unwrap(), 20000000);
                        assert_eq!(new_state2, src.state());
                        assert_eq!(src.req(&3, &30, &300).unwrap(), 300000000000);
                        assert_eq!(new_state2, src.state());
                        assert_eq!(src.req(&4, &40, &400).unwrap(), 444);
                        assert_eq!(new_state2, src.state());
                        assert!(src.req(&5, &50, &500).is_err());
                        assert_eq!(new_state2, src.state());

                        src.req(&1, &10, &100).unwrap()
                    },
                );
                assert_eq!(res, 42);
                assert_eq!(src.req(&1, &10, &100).unwrap(), 1001);
                assert_eq!(new_state, src.state());

                res
            },
        );
        assert_eq!(res, 42);

        assert_eq!(src.req(&1, &10, &100).unwrap(), 1000);
        assert_eq!(state, src.state());
        assert_eq!(src.req(&2, &20, &200).unwrap(), 20000000);
        assert_eq!(state, src.state());
        assert_eq!(src.req(&3, &30, &300).unwrap(), 300000000000);
        assert_eq!(state, src.state());
        assert!(src.req(&4, &40, &400).is_err());
        assert_eq!(state, src.state());
        assert!(src.req(&5, &50, &500).is_err());
        assert_eq!(state, src.state());
    }

    #[rstest]
    fn test_overridable_3args_clone(src_3args: OnMemorySrc3Args<u64, u64, u64, u64>) {
        let mut src = src_3args.overridable::<u64, u64, u64, u64>("overridable");
        let src2 = src.clone_without_override();

        assert_ne!(src.id(), src2.id());

        assert_eq!(src.req(&1, &10, &100).unwrap(), 1000);
        assert_eq!(src2.req(&1, &10, &100).unwrap(), 1000);
        assert_eq!(src.req(&2, &20, &200).unwrap(), 20000000);
        assert_eq!(src2.req(&2, &20, &200).unwrap(), 20000000);

        src.temp_override(
            hashmap! {
                1 => hashmap!{10 => hashmap!{100 => 1, 400 => 4}, 4 => hashmap!{400 => 44}},
                4 => hashmap!{40 => hashmap!{400 => 444}},
            },
            |src| {
                assert_eq!(src.req(&1, &10, &100).unwrap(), 1);
                assert_eq!(src.req(&2, &20, &200).unwrap(), 20000000);
                assert_eq!(src2.req(&1, &10, &100).unwrap(), 1000);
                assert_eq!(src2.req(&2, &20, &200).unwrap(), 20000000);
            },
        );
    }

    #[rstest]
    fn test_overridable_3args_state_change(src_3args: OnMemorySrc3Args<u64, u64, u64, u64>) {
        let mut src = src_3args.overridable::<u64, u64, u64, u64>("overridable");

        let state = src.state();
        assert_eq!(src.req(&1, &10, &100).unwrap(), 1000);
        assert_eq!(state, src.state());

        src.inner_mut().insert(1, 10, 100, 1001);
        assert_eq!(src.req(&1, &10, &100).unwrap(), 1001);
        assert_ne!(state, src.state());
    }

    #[rstest]
    fn test_overridable_3args_tree(src_3args: OnMemorySrc3Args<u64, u64, u64, u64>) {
        let src = src_3args.overridable::<u64, u64, u64, u64>("overridable");
        let Tree::Branch {
            desc,
            id,
            state,
            children,
        } = src.tree()
        else {
            panic!()
        };
        assert_eq!(desc, "overridable");
        assert_eq!(id, src.id());
        assert_eq!(state, src.state());
        assert_eq!(children.len(), 1);
        assert_eq!(children.iter().next().unwrap(), &src.inner().tree());
    }

    #[rstest]
    fn test_overridable_3args_persistent_override(src_3args: OnMemorySrc3Args<u64, u64, u64, u64>) {
        let src = src_3args.overridable::<u64, u64, u64, u64>("overridable");
        let overriden = src.persistent_override(hashmap! {
            1 => hashmap!{10 => hashmap!{100 => 1001, 400 => 4}, 4 => hashmap!{400 => 44}},
            4 => hashmap!{40 => hashmap!{400 => 444}},
        });

        assert_eq!(overriden.req(&1, &10, &100).unwrap(), 1001);
        assert_eq!(overriden.req(&2, &20, &200).unwrap(), 20000000);
        assert_eq!(overriden.req(&3, &30, &300).unwrap(), 300000000000);
        assert_eq!(overriden.req(&4, &40, &400).unwrap(), 444);
        assert!(overriden.req(&5, &50, &500).is_err());
    }
}
