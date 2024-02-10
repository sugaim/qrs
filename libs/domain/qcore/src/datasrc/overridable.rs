use std::{
    borrow::Borrow,
    collections::{HashMap, VecDeque},
    hash::Hash,
    sync::{Arc, Mutex},
};

use maplit::btreeset;
use qcore_derive::Node;

use super::{
    node::DataSrc2Args, private::_UnaryPassThroughNode, snapshot::TakeSnapshot3Args, DataSrc,
    DataSrc3Args, Node, NodeId, NodeInfo, NodeStateId, TakeSnapshot, TakeSnapshot2Args, Tree,
};

// -----------------------------------------------------------------------------
// _Node
//
#[derive(Debug)]
struct _Node<S> {
    src: S,
    info: NodeInfo,
    // state ids for override layers
    states: Mutex<VecDeque<NodeStateId>>,
    tail_state: Mutex<NodeStateId>,
}

//
// construction
//
impl<S: Node> _Node<S> {
    pub fn new(desc: impl Into<String>, src: S) -> Self {
        Self {
            src,
            info: NodeInfo::new(desc),
            states: Mutex::new(VecDeque::new()),
            tail_state: Mutex::new(NodeStateId::gen()),
        }
    }
}

//
// methods
//
impl<S> _Node<S> {
    fn pop(&self) {
        if let Some(state) = self.states.lock().unwrap().pop_back() {
            *self.tail_state.lock().unwrap() = state;
        } else {
            // maybe unreachable
            *self.tail_state.lock().unwrap() = NodeStateId::gen();
        }
    }
    fn push(&self) {
        let mut state = self.tail_state.lock().unwrap();
        self.states.lock().unwrap().push_back(*state);
        *state = NodeStateId::gen();
    }
}

impl<S: Node> Node for _Node<S> {
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
        self.info.set_state(NodeStateId::combine(
            &self.tail_state.lock().unwrap(),
            [state],
        ));
        self.info.notify_all();
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
    pub fn with_layer(desc: impl Into<String>, src: S, layer: HashMap<K, V>) -> Self {
        let info = NodeInfo::new(desc);
        let core = Arc::new(_UnaryPassThroughNode { src, info });
        let subsc = Arc::downgrade(&core);
        core.src.accept_subscriber(subsc);
        Self {
            core,
            layer: Arc::new(layer),
        }
    }
}

impl<S, K, V> Clone for Overriden<S, K, V> {
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

    fn req(&self, key: &Q) -> Result<(super::NodeStateId, Self::Output), Self::Err> {
        if let Some(val) = self.layer.get(key) {
            return Ok((self.core.info.state(), val.clone().into()));
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
        Ok(Overriden::with_layer(
            self.core.info.desc(),
            snapshot,
            layer,
        ))
    }
}

// -----------------------------------------------------------------------------
// Overridable
//
#[derive(Debug, Node)]
#[node(transparent = "core")]
pub struct Overridable<S, K, V> {
    core: Arc<_Node<S>>,
    layers: Arc<Mutex<Vec<HashMap<K, V>>>>,
}

//
// construction
//
impl<S: Node, K, V> Overridable<S, K, V> {
    pub fn _new(desc: impl Into<String>, src: S, layers: Vec<HashMap<K, V>>) -> Self {
        let core = Arc::new(_Node::new(desc, src));
        for _ in 0..layers.len() {
            core.push();
        }
        let subsc = Arc::downgrade(&core);
        core.src.accept_subscriber(subsc);
        let layers = Arc::new(Mutex::new(layers));
        Self { core, layers }
    }
    pub fn with_layer(desc: impl Into<String>, src: S, layer: HashMap<K, V>) -> Self {
        Self::_new(desc, src, vec![layer])
    }
    pub fn new(desc: impl Into<String>, src: S) -> Self {
        Self::_new(desc, src, Vec::new())
    }
}

impl<S, K, V> Clone for Overridable<S, K, V> {
    fn clone(&self) -> Self {
        Self {
            core: self.core.clone(),
            layers: self.layers.clone(),
        }
    }
}

//
// methods
//
impl<S, K, V> Overridable<S, K, V> {
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
        let layer = self.layers.lock().unwrap();
        for map in layer.iter().rev() {
            if let Some(val) = map.get(key) {
                return Ok((self.core.info.state(), val.clone().into()));
            }
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
        let mut layer = HashMap::new();

        for l in self.layers.lock().unwrap().iter() {
            // extend from lower layer to upper layer
            let contained = items.iter().filter_map(|k| l.get_key_value(k));
            layer.extend(contained.map(|(k, v)| (k.clone(), v.clone())));
        }
        Ok(Overriden::with_layer(
            self.core.info.desc(),
            snapshot,
            layer,
        ))
    }
}

impl<S, K, V> Overridable<S, K, V> {
    pub fn push_layer(&mut self, layer: HashMap<K, V>) {
        self.layers.lock().unwrap().push(layer);
        self.core.states.clear();
        self.core.info.set_state(NodeStateId::gen());
        self.core.info.notify_all();
    }

    pub fn pop_layer(&mut self) -> Option<HashMap<K, V>> {
        let Some(res) = self.layers.lock().unwrap().pop() else {
            return None;
        };
        self.core.states.clear();
        self.core.info.set_state(NodeStateId::gen());
        self.core.info.notify_all();
        Some(res)
    }

    pub fn clear_layers(&mut self) {
        {
            let mut layers = self.layers.lock().unwrap();
            if layers.is_empty() {
                return;
            }
            layers.clear();
        }
        self.core.states.clear();
        self.core.info.set_state(NodeStateId::gen());
        self.core.info.notify_all();
    }

    #[inline]
    pub fn num_layers(&self) -> usize {
        self.layers.lock().unwrap().len()
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
    fn _new(desc: impl Into<String>, src: S, layer: HashMap<K1, HashMap<K2, V>>) -> Self {
        let core = Arc::new(_UnaryPassThroughNode {
            src,
            info: NodeInfo::new(desc),
        });
        let subsc = Arc::downgrade(&core);
        core.src.accept_subscriber(subsc);
        Self {
            core,
            layer: Arc::new(layer),
        }
    }
    pub fn with_layer(desc: impl Into<String>, src: S, layer: HashMap<(K1, K2), V>) -> Self {
        let mut nested = HashMap::new();
        for (k1, k2, v) in layer.into_iter().map(|(k, v)| (k.0, k.1, v)) {
            nested.entry(k1).or_insert_with(HashMap::new).insert(k2, v);
        }
        Self::_new(desc, src, nested)
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
            return Ok((self.core.info.state(), v.clone().into()));
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
        Ok(Overriden2Args::_new(self.core.info.desc(), snapshot, layer))
    }
}

// -----------------------------------------------------------------------------
// Overridable2Args
//
#[derive(Debug, Node)]
#[node(transparent = "core")]
pub struct Overridable2Args<S, K1, K2, V> {
    core: Arc<_UnaryPassThroughNode<S>>,
    layers: Arc<Mutex<Vec<HashMap<K1, HashMap<K2, V>>>>>,
}

//
// construction
//
impl<S: Node, K1, K2, V> Overridable2Args<S, K1, K2, V>
where
    K1: Eq + Hash,
    K2: Eq + Hash,
{
    fn _new(desc: impl Into<String>, src: S, layers: Vec<HashMap<K1, HashMap<K2, V>>>) -> Self {
        let core = Arc::new(_UnaryPassThroughNode {
            src,
            states: StateRecorder::new(Some(64)),
            info: NodeInfo::new(desc),
        });
        let subsc = Arc::downgrade(&core);
        core.info.set_state(
            core.states
                .get_or_gen_unwrapped(&core.src.accept_subscriber(subsc)),
        );
        Self {
            core,
            layers: Arc::new(Mutex::new(layers)),
        }
    }
    pub fn with_layer(desc: impl Into<String>, src: S, layer: HashMap<(K1, K2), V>) -> Self {
        let mut nested = HashMap::new();
        for (k1, k2, v) in layer.into_iter().map(|(k, v)| (k.0, k.1, v)) {
            nested.entry(k1).or_insert_with(HashMap::new).insert(k2, v);
        }
        Self::_new(desc, src, vec![nested])
    }
    pub fn new(desc: impl Into<String>, src: S) -> Self {
        Self::_new(desc, src, Vec::new())
    }
}

impl<S, K1, K2, V> Clone for Overridable2Args<S, K1, K2, V> {
    #[inline]
    fn clone(&self) -> Self {
        Self {
            core: self.core.clone(),
            layers: self.layers.clone(),
        }
    }
}

//
// methods
//
impl<S, K1, K2, V> Overridable2Args<S, K1, K2, V> {
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
        let layer = self.layers.lock().unwrap();
        for map in layer.iter().rev() {
            if let Some(v) = map.get(key1).and_then(|m| m.get(key2)) {
                return Ok((self.core.info.state(), v.clone().into()));
            }
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

        for l in self.layers.lock().unwrap().iter() {
            let contained = items.iter().filter_map(|(k1, k2)| {
                l.get_key_value(k1)
                    .and_then(|(k1, m)| m.get_key_value(k2).map(|(k2, v)| (k1, k2, v)))
            });
            for (k1, k2, v) in contained {
                layer
                    .entry(k1.clone())
                    .or_insert_with(HashMap::new)
                    .insert(k2.clone(), v.clone());
            }
        }
        Ok(Overriden2Args::_new(self.core.info.desc(), snapshot, layer))
    }
}

impl<S, K1, K2, V> Overridable2Args<S, K1, K2, V>
where
    K1: Eq + Hash + Clone,
    K2: Eq + Hash + Clone,
{
    pub fn push_layer(&mut self, layer: HashMap<(K1, K2), V>) {
        let mut nested = HashMap::new();
        for (k1, k2, v) in layer.into_iter().map(|(k, v)| (k.0, k.1, v)) {
            nested.entry(k1).or_insert_with(HashMap::new).insert(k2, v);
        }
        self.layers.lock().unwrap().push(nested);
        self.core.states.clear();
        self.core.info.set_state(NodeStateId::gen());
        self.core.info.notify_all();
    }

    pub fn pop_layer(&mut self) -> Option<HashMap<(K1, K2), V>> {
        let Some(res) = self.layers.lock().unwrap().pop() else {
            return None;
        };
        self.core.states.clear();
        self.core.info.set_state(NodeStateId::gen());
        self.core.info.notify_all();
        Some(
            res.into_iter()
                .flat_map(|(k1, m)| m.into_iter().map(move |(k2, v)| ((k1.clone(), k2), v)))
                .collect(),
        )
    }

    pub fn clear_layers(&mut self) {
        let mut layers = self.layers.lock().unwrap();
        if !layers.is_empty() {
            layers.clear();
            self.core.states.clear();
            self.core.info.set_state(NodeStateId::gen());
            self.core.info.notify_all();
        }
    }

    #[inline]
    pub fn num_layers(&self) -> usize {
        self.layers.lock().unwrap().len()
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
    fn _new(
        desc: impl Into<String>,
        src: S,
        layer: HashMap<K1, HashMap<K2, HashMap<K3, V>>>,
    ) -> Self {
        let info = NodeInfo::new(desc);
        let core = Arc::new(_UnaryPassThroughNode { src, info });
        let subsc = Arc::downgrade(&core);
        core.src.accept_subscriber(subsc);
        Self {
            core,
            layer: Arc::new(layer),
        }
    }
    pub fn new(desc: impl Into<String>, src: S, layer: HashMap<(K1, K2, K3), V>) -> Self {
        let mut nested = HashMap::new();
        for (k1, k2, k3, v) in layer.into_iter().map(|(k, v)| (k.0, k.1, k.2, v)) {
            nested
                .entry(k1)
                .or_insert_with(HashMap::new)
                .entry(k2)
                .or_insert_with(HashMap::new)
                .insert(k3, v);
        }
        Self::_new(desc, src, nested)
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
            return Ok((self.core.info.state(), v.clone().into()));
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
            self.layer.get_key_value(k1).and_then(|(k1, m1)| {
                m1.get_key_value(k2)
                    .and_then(|(k2, m2)| m2.get_key_value(k3).map(|(k3, v)| (k1, k2, k3, v)))
            })
        });
        for (k1, k2, k3, v) in contained {
            layer
                .entry(k1.clone())
                .or_insert_with(HashMap::new)
                .entry(k2.clone())
                .or_insert_with(HashMap::new)
                .insert(k3.clone(), v.clone());
        }
        Ok(Overriden3Args::_new(self.core.info.desc(), snapshot, layer))
    }
}

// -----------------------------------------------------------------------------
// Overridable3Args
//
#[derive(Debug, Node)]
#[node(transparent = "core")]
pub struct Overridable3Args<S, K1, K2, K3, V> {
    core: Arc<_UnaryPassThroughNode<S>>,
    layers: Arc<Mutex<Vec<HashMap<K1, HashMap<K2, HashMap<K3, V>>>>>>,
}

//
// construction
//
impl<S: Node, K1, K2, K3, V> Overridable3Args<S, K1, K2, K3, V>
where
    K1: Eq + Hash,
    K2: Eq + Hash,
    K3: Eq + Hash,
{
    fn _new(
        desc: impl Into<String>,
        src: S,
        layers: Vec<HashMap<K1, HashMap<K2, HashMap<K3, V>>>>,
    ) -> Self {
        let core = Arc::new(_UnaryPassThroughNode {
            src,
            states: StateRecorder::new(Some(64)),
            info: NodeInfo::new(desc),
        });
        let subsc = Arc::downgrade(&core);
        core.info.set_state(
            core.states
                .get_or_gen_unwrapped(&core.src.accept_subscriber(subsc)),
        );
        Self {
            core,
            layers: Arc::new(Mutex::new(layers)),
        }
    }
    pub fn with_layer(desc: impl Into<String>, src: S, layer: HashMap<(K1, K2, K3), V>) -> Self {
        let mut nested = HashMap::new();
        for (k1, k2, k3, v) in layer.into_iter().map(|(k, v)| (k.0, k.1, k.2, v)) {
            nested
                .entry(k1)
                .or_insert_with(HashMap::new)
                .entry(k2)
                .or_insert_with(HashMap::new)
                .insert(k3, v);
        }
        Self::_new(desc, src, vec![nested])
    }
    pub fn new(desc: impl Into<String>, src: S) -> Self {
        Self::_new(desc, src, Vec::new())
    }
}

impl<S, K1, K2, K3, V> Clone for Overridable3Args<S, K1, K2, K3, V> {
    #[inline]
    fn clone(&self) -> Self {
        Self {
            core: self.core.clone(),
            layers: self.layers.clone(),
        }
    }
}

//
// methods
//
impl<S, K1, K2, K3, V> Overridable3Args<S, K1, K2, K3, V> {
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
        let layer = self.layers.lock().unwrap();
        for map in layer.iter().rev() {
            if let Some(v) = map
                .get(key1)
                .and_then(|m1| m1.get(key2))
                .and_then(|m2| m2.get(key3))
            {
                return Ok((self.core.info.state(), v.clone().into()));
            }
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

        for l in self.layers.lock().unwrap().iter() {
            let contained = items.iter().filter_map(|(k1, k2, k3)| {
                l.get_key_value(k1).and_then(|(k1, m1)| {
                    m1.get_key_value(k2)
                        .and_then(|(k2, m2)| m2.get_key_value(k3).map(|(k3, v)| (k1, k2, k3, v)))
                })
            });
            for (k1, k2, k3, v) in contained {
                layer
                    .entry(k1.clone())
                    .or_insert_with(HashMap::new)
                    .entry(k2.clone())
                    .or_insert_with(HashMap::new)
                    .insert(k3.clone(), v.clone());
            }
        }
        Ok(Overriden3Args::_new(self.core.info.desc(), snapshot, layer))
    }
}

impl<S, K1, K2, K3, V> Overridable3Args<S, K1, K2, K3, V>
where
    K1: Eq + Hash + Clone,
    K2: Eq + Hash + Clone,
    K3: Eq + Hash + Clone,
{
    pub fn push_layer(&mut self, layer: HashMap<(K1, K2, K3), V>) {
        let mut nested = HashMap::new();
        for (k1, k2, k3, v) in layer.into_iter().map(|(k, v)| (k.0, k.1, k.2, v)) {
            nested
                .entry(k1)
                .or_insert_with(HashMap::new)
                .entry(k2)
                .or_insert_with(HashMap::new)
                .insert(k3, v);
        }
        self.layers.lock().unwrap().push(nested);
        self.core.states.clear();
        self.core.info.set_state(NodeStateId::gen());
        self.core.info.notify_all();
    }

    pub fn clear_layers(&mut self) {
        let mut layers = self.layers.lock().unwrap();
        if !layers.is_empty() {
            layers.clear();
            self.core.states.clear();
            self.core.info.set_state(NodeStateId::gen());
            self.core.info.notify_all();
        }
    }

    #[inline]
    pub fn num_layers(&self) -> usize {
        self.layers.lock().unwrap().len()
    }
}
