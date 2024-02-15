use std::{
    collections::BTreeSet,
    fmt::Display,
    ops::{BitXor, BitXorAssign},
    sync::{Arc, Mutex, Weak},
};

use derivative::Derivative;
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
// StateId
//
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize)]
pub struct StateId(Uuid);

//
// display, serde
//
impl Display for StateId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.0.fmt(f)
    }
}

//
// construction
//
impl StateId {
    /// Create a new `NodeStateId`
    ///
    /// This function intentionally nullary to ensure
    /// that the `NodeStateId` is almost surely unique.
    pub fn gen() -> Self {
        StateId(Uuid::new_v4())
    }
}

impl BitXor for StateId {
    type Output = Self;

    fn bitxor(self, rhs: Self) -> Self::Output {
        Self(Uuid::from_u128(self.0.as_u128() ^ rhs.0.as_u128()))
    }
}

impl BitXor for &StateId {
    type Output = StateId;

    fn bitxor(self, rhs: Self) -> Self::Output {
        StateId(Uuid::from_u128(self.0.as_u128() ^ rhs.0.as_u128()))
    }
}

impl BitXor<&StateId> for StateId {
    type Output = StateId;

    fn bitxor(self, rhs: &StateId) -> Self::Output {
        StateId(Uuid::from_u128(self.0.as_u128() ^ rhs.0.as_u128()))
    }
}

impl BitXor<StateId> for &StateId {
    type Output = StateId;

    fn bitxor(self, rhs: StateId) -> Self::Output {
        StateId(Uuid::from_u128(self.0.as_u128() ^ rhs.0.as_u128()))
    }
}

impl BitXorAssign for StateId {
    fn bitxor_assign(&mut self, rhs: Self) {
        *self = *self ^ rhs;
    }
}

impl BitXorAssign<&StateId> for StateId {
    fn bitxor_assign(&mut self, rhs: &Self) {
        *self = *self ^ rhs;
    }
}

//
// methods
//
impl StateId {
    pub fn uuid(&self) -> &Uuid {
        &self.0
    }
}

// -----------------------------------------------------------------------------
// PublisherState
//

#[derive(Derivative)]
#[derivative(Debug)]
pub struct PublisherState {
    id: NodeId,
    desc: String,
    state: StateId,
    #[derivative(Debug = "ignore")]
    children: Vec<Weak<Mutex<dyn Listener>>>,
}

//
// construction
//
impl PublisherState {
    pub fn new(desc: impl Into<String>) -> Self {
        Self {
            id: NodeId::gen(),
            desc: desc.into(),
            state: StateId::gen(),
            children: Vec::new(),
        }
    }
}

//
// methods
//
impl PublisherState {
    #[inline]
    pub fn id(&self) -> NodeId {
        self.id
    }
    #[inline]
    pub fn desc(&self) -> &str {
        &self.desc
    }
    #[inline]
    pub fn state(&self) -> StateId {
        self.state
    }
    pub fn set_state(&mut self, state: StateId) {
        self.state = state;

        // notify to all subscribers
        let id = self.id;
        self.children.retain(|child| match child.upgrade() {
            Some(child) => {
                child.lock().unwrap().listen(&id, &state);
                true
            }
            None => false,
        });
    }
    pub fn accept_listener(&mut self, subsc: Weak<Mutex<dyn Listener>>) -> StateId {
        let Some(subsc_id) = subsc.upgrade().map(|s| s.lock().unwrap().id()) else {
            // the subscriber is already dropped. so do nothing
            return self.state();
        };
        self.remove_listener(&subsc_id);
        self.children.push(subsc);
        self.state()
    }
    #[inline]
    pub fn remove_listener(&mut self, subscriber: &NodeId) {
        self.children.retain(|child| match child.upgrade() {
            Some(child) => &child.lock().unwrap().id() != subscriber,
            None => false,
        });
    }
    #[inline]
    pub fn make_tree_as_leaf(&self) -> Tree {
        Tree::Leaf {
            desc: self.desc.to_owned(),
            id: self.id,
            state: self.state(),
        }
    }
    #[inline]
    pub fn make_tree_as_branch(&self, children: BTreeSet<Tree>) -> Tree {
        Tree::Branch {
            desc: self.desc.to_owned(),
            id: self.id,
            state: self.state(),
            children,
        }
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
        state: StateId,
    },
    Branch {
        desc: String,
        id: NodeId,
        state: StateId,
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
pub trait Notifier: 'static + Send + Sync {
    /// Get the `NodeId` of this node.
    /// Implementations must ensure that the `NodeId` is immutable.
    fn id(&self) -> NodeId;

    /// Get the `StateId` of this node.
    fn state(&self) -> StateId;

    /// Get the tree structure of the node
    /// Mainly used for debugging
    fn tree(&self) -> Tree;

    /// Accept subscriber which subscribes the change of this node.
    fn accept_listener(&mut self, subsc: Weak<Mutex<dyn Listener>>);

    /// Remove subscriber which subscribes the change of this node
    fn remove_listener(&mut self, id: &NodeId);
}

impl<P: Notifier> Notifier for Mutex<P> {
    #[inline]
    fn id(&self) -> NodeId {
        self.lock().unwrap().id()
    }

    #[inline]
    fn state(&self) -> StateId {
        self.lock().unwrap().state()
    }

    #[inline]
    fn tree(&self) -> Tree {
        self.lock().unwrap().tree()
    }

    #[inline]
    fn accept_listener(&mut self, subsc: Weak<Mutex<dyn Listener>>) {
        self.lock().unwrap().accept_listener(subsc);
    }

    #[inline]
    fn remove_listener(&mut self, id: &NodeId) {
        self.lock().unwrap().remove_listener(id)
    }
}

impl<P: Notifier> Notifier for Arc<Mutex<P>> {
    #[inline]
    fn id(&self) -> NodeId {
        self.lock().unwrap().id()
    }

    #[inline]
    fn state(&self) -> StateId {
        self.lock().unwrap().state()
    }

    #[inline]
    fn tree(&self) -> Tree {
        self.lock().unwrap().tree()
    }

    #[inline]
    fn accept_listener(&mut self, subsc: Weak<Mutex<dyn Listener>>) {
        self.lock().unwrap().accept_listener(subsc);
    }

    #[inline]
    fn remove_listener(&mut self, id: &NodeId) {
        self.lock().unwrap().remove_listener(id)
    }
}

// -----------------------------------------------------------------------------
// Listener
//

pub trait Listener: 'static + Send + Sync {
    /// Get the `NodeId` of the node
    fn id(&self) -> NodeId;

    /// Accept the state changing event from the publisher
    fn listen(&mut self, publisher: &NodeId, state: &StateId);
}

impl<S: Listener> Listener for Mutex<S> {
    #[inline]
    fn id(&self) -> NodeId {
        self.lock().unwrap().id()
    }

    #[inline]
    fn listen(&mut self, publisher: &NodeId, state: &StateId) {
        self.lock().unwrap().listen(publisher, state)
    }
}

impl<S: Listener> Listener for Arc<Mutex<S>> {
    #[inline]
    fn id(&self) -> NodeId {
        self.lock().unwrap().id()
    }

    #[inline]
    fn listen(&mut self, publisher: &NodeId, state: &StateId) {
        self.lock().unwrap().listen(publisher, state)
    }
}

// -----------------------------------------------------------------------------
// DataSrc
//
pub trait DataSrc: Notifier {
    type Key: ?Sized;
    type Output;
    type Err;

    /// Request data with the given key
    /// In addition to the data, the state id of the node is also returned.
    ///
    /// - `Ok`: when the data is successfully retrieved
    /// - `Err`: when the data is not found or some error occurred
    fn req(&self, key: &Self::Key) -> Result<Self::Output, Self::Err>;

    /// Map the output of this data source
    fn map<F>(self, desc: impl Into<String>, f: F) -> Map<Self, F>
    where
        Self: Sized,
        F: 'static + Send + Sync + Fn(Self::Output) -> Self::Output,
    {
        Map::new(desc, self, f)
    }

    /// Map the error of this data source
    fn map_err<F, E>(self, desc: impl Into<String>, f: F) -> MapErr<Self, F>
    where
        Self: Sized,
        F: 'static + Send + Sync + Fn(Self::Err) -> E,
    {
        MapErr::new(desc, self, f)
    }

    /// Convert the output and error of this data source
    fn convert<F, O, E>(self, desc: impl Into<String>, f: F) -> Convert<Self, F>
    where
        Self: Sized,
        F: 'static + Send + Sync + Fn(&Self::Key, Result<Self::Output, Self::Err>) -> Result<O, E>,
    {
        Convert::new(desc, self, f)
    }

    /// Add a logger to data source
    fn with_logger<L>(self, desc: impl Into<String>, logger: L) -> WithLogger<Self, L>
    where
        Self: Sized,
        L: Fn(&Self::Key, &Result<Self::Output, Self::Err>) + 'static,
    {
        WithLogger::new(desc, self, logger)
    }
}

impl<T: DataSrc> DataSrc for Mutex<T> {
    type Key = T::Key;
    type Output = T::Output;
    type Err = T::Err;

    #[inline]
    fn req(&self, key: &Self::Key) -> Result<Self::Output, Self::Err> {
        self.lock().unwrap().req(key)
    }
}

impl<T: DataSrc> DataSrc for Arc<Mutex<T>> {
    type Key = T::Key;
    type Output = T::Output;
    type Err = T::Err;

    #[inline]
    fn req(&self, key: &Self::Key) -> Result<Self::Output, Self::Err> {
        self.lock().unwrap().req(key)
    }
}

// -----------------------------------------------------------------------------
// DataSrc2Args
//
pub trait DataSrc2Args: Notifier {
    type Key1: ?Sized;
    type Key2: ?Sized;
    type Output;
    type Err;

    /// Request data with the given keys
    /// In addition to the data, the state id of the node is also returned.
    ///
    /// - `Ok`: when the data is successfully retrieved
    /// - `Err`: when the data is not found or some error occurred
    fn req(&self, key1: &Self::Key1, key2: &Self::Key2) -> Result<Self::Output, Self::Err>;

    /// Map the output of this data source
    fn map<F>(self, desc: impl Into<String>, f: F) -> Map<Self, F>
    where
        Self: Sized,
        F: 'static + Send + Sync + Fn(Self::Output) -> Self::Output,
    {
        Map::new(desc, self, f)
    }

    /// Map the error of this data source
    fn map_err<F, E>(self, desc: impl Into<String>, f: F) -> MapErr<Self, F>
    where
        Self: Sized,
        F: 'static + Send + Sync + Fn(Self::Err) -> E,
    {
        MapErr::new(desc, self, f)
    }

    /// Convert the output and error of this data source
    fn convert<F, O, E>(self, desc: impl Into<String>, f: F) -> Convert<Self, F>
    where
        Self: Sized,
        F: 'static
            + Send
            + Sync
            + Fn(&Self::Key1, &Self::Key2, Result<Self::Output, Self::Err>) -> Result<O, E>,
    {
        Convert::new(desc, self, f)
    }

    /// Add a logger to data source
    fn with_logger<L>(self, desc: impl Into<String>, logger: L) -> WithLogger<Self, L>
    where
        Self: Sized,
        L: Fn(&Self::Key1, &Self::Key2, &Result<Self::Output, Self::Err>) + 'static,
    {
        WithLogger::new(desc, self, logger)
    }
}

impl<T: DataSrc2Args> DataSrc2Args for Mutex<T> {
    type Key1 = T::Key1;
    type Key2 = T::Key2;
    type Output = T::Output;
    type Err = T::Err;

    #[inline]
    fn req(&self, key1: &Self::Key1, key2: &Self::Key2) -> Result<Self::Output, Self::Err> {
        self.lock().unwrap().req(key1, key2)
    }
}

impl<T: DataSrc2Args> DataSrc2Args for Arc<Mutex<T>> {
    type Key1 = T::Key1;
    type Key2 = T::Key2;
    type Output = T::Output;
    type Err = T::Err;

    #[inline]
    fn req(&self, key1: &Self::Key1, key2: &Self::Key2) -> Result<Self::Output, Self::Err> {
        self.lock().unwrap().req(key1, key2)
    }
}

// -----------------------------------------------------------------------------
// DataSrc3Args
//
pub trait DataSrc3Args: Notifier {
    type Key1: ?Sized;
    type Key2: ?Sized;
    type Key3: ?Sized;
    type Output;
    type Err;

    /// Request data with the given keys
    /// In addition to the data, the state id of the node is also returned.
    ///
    /// - `Ok`: when the data is successfully retrieved
    /// - `Err`: when the data is not found or some error occurred
    fn req(
        &self,
        key1: &Self::Key1,
        key2: &Self::Key2,
        key3: &Self::Key3,
    ) -> Result<Self::Output, Self::Err>;

    /// Map the output of this data source
    fn map<F>(self, desc: impl Into<String>, f: F) -> Map<Self, F>
    where
        Self: Sized,
        F: 'static + Send + Sync + Fn(Self::Output) -> Self::Output,
    {
        Map::new(desc, self, f)
    }

    /// Map the error of this data source
    fn map_err<F, E>(self, desc: impl Into<String>, f: F) -> MapErr<Self, F>
    where
        Self: Sized,
        F: 'static + Send + Sync + Fn(Self::Err) -> E,
    {
        MapErr::new(desc, self, f)
    }

    /// Convert the output and error of this data source
    fn convert<F, O, E>(self, desc: impl Into<String>, f: F) -> Convert<Self, F>
    where
        Self: Sized,
        F: 'static
            + Send
            + Sync
            + Fn(
                &Self::Key1,
                &Self::Key2,
                &Self::Key3,
                Result<Self::Output, Self::Err>,
            ) -> Result<O, E>,
    {
        Convert::new(desc, self, f)
    }

    /// Add a logger to data source
    fn with_logger<L>(self, desc: impl Into<String>, logger: L) -> WithLogger<Self, L>
    where
        Self: Sized,
        L: Fn(&Self::Key1, &Self::Key2, &Self::Key3, &Result<Self::Output, Self::Err>) + 'static,
    {
        WithLogger::new(desc, self, logger)
    }
}

impl<T: DataSrc3Args> DataSrc3Args for Mutex<T> {
    type Key1 = T::Key1;
    type Key2 = T::Key2;
    type Key3 = T::Key3;
    type Output = T::Output;
    type Err = T::Err;

    #[inline]
    fn req(
        &self,
        key1: &Self::Key1,
        key2: &Self::Key2,
        key3: &Self::Key3,
    ) -> Result<Self::Output, Self::Err> {
        self.lock().unwrap().req(key1, key2, key3)
    }
}

impl<T: DataSrc3Args> DataSrc3Args for Arc<Mutex<T>> {
    type Key1 = T::Key1;
    type Key2 = T::Key2;
    type Key3 = T::Key3;
    type Output = T::Output;
    type Err = T::Err;

    #[inline]
    fn req(
        &self,
        key1: &Self::Key1,
        key2: &Self::Key2,
        key3: &Self::Key3,
    ) -> Result<Self::Output, Self::Err> {
        self.lock().unwrap().req(key1, key2, key3)
    }
}
