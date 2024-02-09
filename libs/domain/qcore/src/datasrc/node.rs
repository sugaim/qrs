use std::{
    collections::{BTreeSet, VecDeque},
    fmt::Display,
    sync::{Mutex, Weak},
};

use derivative::Derivative;
use dyn_clone::DynClone;
use serde::Serialize;
use uuid::Uuid;

use super::{Convert, Map, MapErr, WithLogger};

// -----------------------------------------------------------------------------
// NodeId
//
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize)]
pub struct NodeId(Uuid);

//
// display, serde
//
impl Display for NodeId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.0.fmt(f)
    }
}

//
// construction
//
impl NodeId {
    /// Create a new `NodeId`
    ///
    /// This function intentionally nullary to ensure
    /// that the `NodeId` is almost surely unique.
    pub fn gen() -> Self {
        NodeId(Uuid::new_v4())
    }
}

//
// methods
//
impl NodeId {
    pub fn uuid(&self) -> &Uuid {
        &self.0
    }
}

// -----------------------------------------------------------------------------
// NodeStateId
//
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize)]
pub struct NodeStateId(Uuid);

//
// display, serde
//
impl Display for NodeStateId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.0.fmt(f)
    }
}

//
// construction
//
impl NodeStateId {
    /// Create a new `NodeStateId`
    ///
    /// This function intentionally nullary to ensure
    /// that the `NodeStateId` is almost surely unique.
    pub fn gen() -> Self {
        NodeStateId(Uuid::new_v4())
    }
}

//
// methods
//
impl NodeStateId {
    pub fn uuid(&self) -> &Uuid {
        &self.0
    }
}

// -----------------------------------------------------------------------------
// NodeInfo
//

/// A collection of data which may be necessary for typical implementation of `Node`
#[derive(Derivative)]
#[derivative(Debug)]
pub struct NodeInfo {
    id: NodeId,
    desc: String,
    state: Mutex<NodeStateId>,
    #[derivative(Debug = "ignore")]
    children: Mutex<Vec<Weak<dyn Node>>>,
}

//
// construction
//
impl NodeInfo {
    pub fn new(desc: impl Into<String>) -> Self {
        Self {
            id: NodeId::gen(),
            desc: desc.into(),
            state: Mutex::new(NodeStateId::gen()),
            children: Mutex::new(Vec::new()),
        }
    }
}

//
// methods
//
impl NodeInfo {
    #[inline]
    pub fn id(&self) -> NodeId {
        self.id
    }
    #[inline]
    pub fn state(&self) -> NodeStateId {
        self.state.lock().unwrap().clone()
    }
    #[inline]
    pub fn set_state(&self, state: NodeStateId) {
        *self.state.lock().unwrap() = state;
    }
    #[inline]
    pub fn accept_subscriber(&self, subscriber: Weak<dyn Node>) -> NodeStateId {
        self.children.lock().unwrap().push(subscriber);
        self.state()
    }
    #[inline]
    pub fn desc(&self) -> &str {
        &self.desc
    }
    pub fn remove_subscriber(&self, subscriber: &NodeId) {
        self.children
            .lock()
            .unwrap()
            .retain(|child| match &child.upgrade().map(|c| c.id()) {
                Some(id) => id != subscriber,
                None => false,
            });
    }
    pub fn notify_all(&self) {
        let id = self.id();
        let state = self.state();
        let mut children = self.children.lock().unwrap();
        children.retain(|child| match &child.upgrade() {
            Some(child) => {
                child.accept_state(&id, &state);
                true
            }
            None => false,
        });
    }
    #[inline]
    pub fn make_tree_as_leaf(&self) -> Tree {
        Tree::Leaf {
            desc: self.desc().to_string(),
            id: self.id(),
            state: self.state(),
        }
    }
    #[inline]
    pub fn make_tree_as_branch(&self, children: BTreeSet<Tree>) -> Tree {
        Tree::Branch {
            desc: self.desc().to_string(),
            id: self.id(),
            state: self.state(),
            children,
        }
    }
}

// -----------------------------------------------------------------------------
// StateRecorder
//
#[derive(Debug)]
pub struct StateRecorder<K> {
    max: Option<u64>,
    data: Mutex<VecDeque<(K, NodeStateId)>>,
}

//
// construction
//
impl<K> StateRecorder<K> {
    pub fn new(max_capacity: Option<u64>) -> Self {
        Self {
            max: max_capacity,
            data: Mutex::new(VecDeque::new()),
        }
    }
}

//
// methods
//
impl<K: Eq + Clone> StateRecorder<K> {
    /// Get the state id of the node with the given key.
    ///
    /// - `Ok`: when state is already recorded
    /// - `Err`: when state is not recorded. New state id is generated and returned.
    pub fn get_or_gen(&self, key: &K) -> Result<NodeStateId, NodeStateId> {
        let mut data = self.data.lock().unwrap();
        if let Some(id) = data.iter().rev().find(|(k, _)| k == key).map(|(_, id)| *id) {
            return Ok(id);
        }
        let new = NodeStateId::gen();
        data.push_back((key.clone(), new));
        if let Some(max) = self.max {
            let max = max as usize;
            while max < data.len() {
                data.pop_front();
            }
        }
        Err(new)
    }

    pub fn get_or_gen_unwrapped(&self, key: &K) -> NodeStateId {
        self.get_or_gen(key).unwrap_or_else(|id| id)
    }
}

// -----------------------------------------------------------------------------
// Tree
//
#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize)]
pub enum Tree {
    Leaf {
        desc: String,
        id: NodeId,
        state: NodeStateId,
    },
    Branch {
        desc: String,
        id: NodeId,
        state: NodeStateId,
        children: BTreeSet<Tree>,
    },
}

// -----------------------------------------------------------------------------
// Node
//

/// Node of dependency graph
///
/// Each node should have a immutable `NodeId`and a mutable `NodeStateId`.
/// When the state changing event occurs, the node should notify it
/// to all subscribers.
pub trait Node: 'static {
    /// Get the `NodeId` of the node
    fn id(&self) -> NodeId;

    /// Get the tree structure of the node
    /// Mainly used for debugging
    fn tree(&self) -> Tree;

    /// Behavior as publisher. Accept subscriber which subscribes the change of this node.
    /// Return the state id of this node when the subscriber is accepted.
    fn accept_subscriber(&self, subscriber: Weak<dyn Node>) -> NodeStateId;

    /// Behavior as publisher. Remove subscriber which subscribes the change of this node
    fn remove_subscriber(&self, subscriber: &NodeId);

    /// Behavior as subscriber. Accept the state changing event from the publisher
    fn accept_state(&self, publisher: &NodeId, state: &NodeStateId);
}

// -----------------------------------------------------------------------------
// DataSrc
//
pub trait DataSrc<K: ?Sized>: DynClone + Node {
    type Output;
    type Err;

    /// Request data with the given key
    /// In addition to the data, the state id of the node is also returned.
    ///
    /// - `Ok`: when the data is successfully retrieved
    /// - `Err`: when the data is not found or some error occurred
    fn req(&self, key: &K) -> Result<(NodeStateId, Self::Output), Self::Err>;

    /// Map the output of this data source
    fn map<F>(self, f: F, desc: impl Into<String>) -> Map<Self, F>
    where
        Self: Sized,
        F: Fn(Self::Output) -> Self::Output + 'static,
    {
        Map::new(desc, self, f)
    }

    /// Map the error of this data source
    fn map_err<F, E>(self, f: F, desc: impl Into<String>) -> MapErr<Self, F>
    where
        Self: Sized,
        F: Fn(Self::Err) -> E + 'static,
    {
        MapErr::new(desc, self, f)
    }

    /// Convert the output and error of this data source
    fn convert<F, O, E>(self, f: F, desc: impl Into<String>) -> Convert<Self, F>
    where
        Self: Sized,
        F: Fn(Result<Self::Output, Self::Err>) -> Result<O, E> + 'static,
    {
        Convert::new(desc, self, f)
    }

    /// Add a logger to data source
    fn with_logger<L>(self, logger: L, desc: impl Into<String>) -> WithLogger<Self, L>
    where
        Self: Sized,
        L: Fn(&K, &Result<(NodeStateId, Self::Output), Self::Err>) + 'static,
    {
        WithLogger::new(desc, self, logger)
    }
}

// -----------------------------------------------------------------------------
// DataSrc2Args
//
pub trait DataSrc2Args<K1: ?Sized, K2: ?Sized>: DynClone + Node {
    type Output;
    type Err;

    /// Request data with the given keys
    /// In addition to the data, the state id of the node is also returned.
    ///
    /// - `Ok`: when the data is successfully retrieved
    /// - `Err`: when the data is not found or some error occurred
    fn req(&self, key1: &K1, key2: &K2) -> Result<(NodeStateId, Self::Output), Self::Err>;

    /// Map the output of this data source
    fn map<F>(self, f: F, desc: impl Into<String>) -> Map<Self, F>
    where
        Self: Sized,
        F: Fn(Self::Output) -> Self::Output + 'static,
    {
        Map::new(desc, self, f)
    }

    /// Map the error of this data source
    fn map_err<F, E>(self, f: F, desc: impl Into<String>) -> MapErr<Self, F>
    where
        Self: Sized,
        F: Fn(Self::Err) -> E + 'static,
    {
        MapErr::new(desc, self, f)
    }

    /// Convert the output and error of this data source
    fn convert<F, O, E>(self, f: F, desc: impl Into<String>) -> Convert<Self, F>
    where
        Self: Sized,
        F: Fn(Result<Self::Output, Self::Err>) -> Result<O, E> + 'static,
    {
        Convert::new(desc, self, f)
    }

    /// Add a logger to data source
    fn with_logger<L>(self, logger: L, desc: impl Into<String>) -> WithLogger<Self, L>
    where
        Self: Sized,
        L: Fn(&K1, &K2, &Result<(NodeStateId, Self::Output), Self::Err>) + 'static,
    {
        WithLogger::new(desc, self, logger)
    }
}

// -----------------------------------------------------------------------------
// DataSrc3Args
//
pub trait DataSrc3Args<K1: ?Sized, K2: ?Sized, K3: ?Sized>: DynClone + Node {
    type Output;
    type Err;

    /// Request data with the given keys
    /// In addition to the data, the state id of the node is also returned.
    ///
    /// - `Ok`: when the data is successfully retrieved
    /// - `Err`: when the data is not found or some error occurred
    fn req(
        &self,
        key1: &K1,
        key2: &K2,
        key3: &K3,
    ) -> Result<(NodeStateId, Self::Output), Self::Err>;

    /// Map the output of this data source
    fn map<F>(self, f: F, desc: impl Into<String>) -> Map<Self, F>
    where
        Self: Sized,
        F: Fn(Self::Output) -> Self::Output + 'static,
    {
        Map::new(desc, self, f)
    }

    /// Map the error of this data source
    fn map_err<F, E>(self, f: F, desc: impl Into<String>) -> MapErr<Self, F>
    where
        Self: Sized,
        F: Fn(Self::Err) -> E + 'static,
    {
        MapErr::new(desc, self, f)
    }

    /// Convert the output and error of this data source
    fn convert<F, O, E>(self, f: F, desc: impl Into<String>) -> Convert<Self, F>
    where
        Self: Sized,
        F: Fn(Result<Self::Output, Self::Err>) -> Result<O, E> + 'static,
    {
        Convert::new(desc, self, f)
    }

    /// Add a logger to data source
    fn with_logger<L>(self, logger: L, desc: impl Into<String>) -> WithLogger<Self, L>
    where
        Self: Sized,
        L: Fn(&K1, &K2, &K3, &Result<(NodeStateId, Self::Output), Self::Err>) + 'static,
    {
        WithLogger::new(desc, self, logger)
    }
}
