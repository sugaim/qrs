use std::sync::Arc;

use maplit::btreeset;
use qcore_derive::Node;

use super::{
    node::DataSrc2Args, snapshot::TakeSnapshot3Args, DataSrc, DataSrc3Args, Node, NodeId, NodeInfo,
    NodeStateId, StateRecorder, TakeSnapshot, TakeSnapshot2Args, Tree,
};

// -----------------------------------------------------------------------------
// _Node
//
#[derive(Debug)]
pub struct _Node<S> {
    src: S,
    states: StateRecorder<NodeStateId>,
    info: NodeInfo,
}

//
// methods
//
impl<S: Node> Node for _Node<S> {
    #[inline]
    fn id(&self) -> NodeId {
        self.info.id()
    }

    #[inline]
    fn tree(&self) -> super::Tree {
        Tree::Branch {
            desc: self.info.desc().to_owned(),
            id: self.id(),
            state: self.info.state(),
            children: btreeset![self.src.tree()],
        }
    }

    #[inline]
    fn accept_subscriber(&self, subscriber: std::sync::Weak<dyn Node>) -> super::NodeStateId {
        self.info.accept_subscriber(subscriber)
    }

    #[inline]
    fn remove_subscriber(&self, subscriber: &NodeId) {
        self.info.remove_subscriber(subscriber)
    }

    #[inline]
    fn accept_state(&self, publisher: &super::NodeId, state: &super::NodeStateId) {
        if publisher != &self.src.id() {
            return;
        }
        let state = self.states.get_or_gen_unwrapped(state);
        self.info.set_state(state);
        self.info.notify_all();
    }
}

// -----------------------------------------------------------------------------
// Map
//
#[derive(Debug, Node)]
#[node(transparent = "core")]
pub struct Map<S, F> {
    core: Arc<_Node<S>>,
    f: Arc<F>,
}

//
// construction
//
impl<S, F> Clone for Map<S, F> {
    #[inline]
    fn clone(&self) -> Self {
        Self {
            core: self.core.clone(),
            f: self.f.clone(),
        }
    }
}

impl<S: Node, F: 'static> Map<S, F> {
    pub fn new(desc: impl Into<String>, src: S, f: F) -> Self {
        let info = NodeInfo::new(desc);
        let states = StateRecorder::new(Some(64));
        let core = Arc::new(_Node { src, states, info });
        let subs = Arc::downgrade(&core);
        core.src.accept_subscriber(subs);
        Self {
            core,
            f: Arc::new(f),
        }
    }
}

//
// methods
//
impl<S, F> Map<S, F> {
    pub fn downstream(&self) -> &S {
        &self.core.src
    }
}

impl<K, S, F, O> DataSrc<K> for Map<S, F>
where
    K: ?Sized,
    S: DataSrc<K>,
    F: Fn(S::Output) -> O + 'static,
{
    type Output = O;
    type Err = S::Err;

    #[inline]
    fn req(&self, key: &K) -> Result<(NodeStateId, Self::Output), Self::Err> {
        let (_, output) = self.core.src.req(key)?;
        Ok((self.core.info.state(), (self.f)(output)))
    }
}

impl<K1, K2, S, F, O> DataSrc2Args<K1, K2> for Map<S, F>
where
    K1: ?Sized,
    K2: ?Sized,
    S: DataSrc2Args<K1, K2>,
    F: Fn(S::Output) -> O + 'static,
{
    type Output = O;
    type Err = S::Err;

    #[inline]
    fn req(&self, key1: &K1, key2: &K2) -> Result<(NodeStateId, Self::Output), Self::Err> {
        let (_, output) = self.core.src.req(key1, key2)?;
        Ok((self.core.info.state(), (self.f)(output)))
    }
}

impl<K1, K2, K3, S, F, O> DataSrc3Args<K1, K2, K3> for Map<S, F>
where
    K1: ?Sized,
    K2: ?Sized,
    K3: ?Sized,
    S: DataSrc3Args<K1, K2, K3>,
    F: Fn(S::Output) -> O + 'static,
{
    type Output = O;
    type Err = S::Err;

    #[inline]
    fn req(
        &self,
        key1: &K1,
        key2: &K2,
        key3: &K3,
    ) -> Result<(NodeStateId, Self::Output), Self::Err> {
        let (_, output) = self.core.src.req(key1, key2, key3)?;
        Ok((self.core.info.state(), (self.f)(output)))
    }
}

impl<K, S, F, O> TakeSnapshot<K> for Map<S, F>
where
    K: ?Sized,
    S: TakeSnapshot<K>,
    F: Fn(S::Output) -> O + 'static,
{
    type SnapShot = Map<S::SnapShot, F>;
    type SnapShotErr = S::SnapShotErr;

    fn take_snapshot<'a, It>(&self, keys: It) -> Result<Self::SnapShot, Self::SnapShotErr>
    where
        It: IntoIterator<Item = &'a K>,
        K: 'a,
    {
        let snap = self.core.src.take_snapshot(keys)?;
        Ok(Map {
            core: Arc::new(_Node {
                src: snap,
                states: StateRecorder::new(Some(64)),
                info: NodeInfo::new(self.core.info.desc()),
            }),
            f: self.f.clone(),
        })
    }
}

impl<K1, K2, S, F, O> TakeSnapshot2Args<K1, K2> for Map<S, F>
where
    K1: ?Sized,
    K2: ?Sized,
    S: TakeSnapshot2Args<K1, K2>,
    F: Fn(S::Output) -> O + 'static,
{
    type SnapShot = Map<S::SnapShot, F>;
    type SnapShotErr = S::SnapShotErr;

    fn take_snapshot<'a, It>(&self, keys: It) -> Result<Self::SnapShot, Self::SnapShotErr>
    where
        It: IntoIterator<Item = (&'a K1, &'a K2)>,
        K1: 'a,
        K2: 'a,
    {
        let snap = self.core.src.take_snapshot(keys)?;
        Ok(Map {
            core: Arc::new(_Node {
                src: snap,
                states: StateRecorder::new(Some(64)),
                info: NodeInfo::new(self.core.info.desc()),
            }),
            f: self.f.clone(),
        })
    }
}

impl<K1, K2, K3, S, F, O> TakeSnapshot3Args<K1, K2, K3> for Map<S, F>
where
    K1: ?Sized,
    K2: ?Sized,
    K3: ?Sized,
    S: TakeSnapshot3Args<K1, K2, K3>,
    F: Fn(S::Output) -> O + 'static,
{
    type SnapShot = Map<S::SnapShot, F>;
    type SnapShotErr = S::SnapShotErr;

    fn take_snapshot<'a, It>(&self, keys: It) -> Result<Self::SnapShot, Self::SnapShotErr>
    where
        It: IntoIterator<Item = (&'a K1, &'a K2, &'a K3)>,
        K1: 'a,
        K2: 'a,
        K3: 'a,
    {
        let snap = self.core.src.take_snapshot(keys)?;
        Ok(Map {
            core: Arc::new(_Node {
                src: snap,
                states: StateRecorder::new(Some(64)),
                info: NodeInfo::new(self.core.info.desc()),
            }),
            f: self.f.clone(),
        })
    }
}

// -----------------------------------------------------------------------------
// MapErr
//
#[derive(Debug, Node)]
#[node(transparent = "core")]
pub struct MapErr<S, F> {
    core: Arc<_Node<S>>,
    f: Arc<F>,
}

//
// construction
//
impl<S, F> Clone for MapErr<S, F> {
    #[inline]
    fn clone(&self) -> Self {
        Self {
            core: self.core.clone(),
            f: self.f.clone(),
        }
    }
}

impl<S: Node, F: 'static> MapErr<S, F> {
    pub fn new(desc: impl Into<String>, src: S, f: F) -> Self {
        let info = NodeInfo::new(desc);
        let states = StateRecorder::new(Some(64));
        let core = Arc::new(_Node { src, states, info });
        let subs = Arc::downgrade(&core);
        core.src.accept_subscriber(subs);
        Self {
            core,
            f: Arc::new(f),
        }
    }
}

//
// methods
//
impl<S, F> MapErr<S, F> {
    pub fn downstream(&self) -> &S {
        &self.core.src
    }
}

impl<K, S, F, E> DataSrc<K> for MapErr<S, F>
where
    K: ?Sized,
    S: DataSrc<K>,
    F: Fn(S::Err) -> E + 'static,
{
    type Output = S::Output;
    type Err = E;

    #[inline]
    fn req(&self, key: &K) -> Result<(NodeStateId, Self::Output), Self::Err> {
        match self.core.src.req(key) {
            Ok((state, output)) => Ok((state, output)),
            Err(err) => Err((self.f)(err)),
        }
    }
}

impl<K1, K2, S, F, E> DataSrc2Args<K1, K2> for MapErr<S, F>
where
    K1: ?Sized,
    K2: ?Sized,
    S: DataSrc2Args<K1, K2>,
    F: Fn(S::Err) -> E + 'static,
{
    type Output = S::Output;
    type Err = E;

    #[inline]
    fn req(&self, key1: &K1, key2: &K2) -> Result<(NodeStateId, Self::Output), Self::Err> {
        match self.core.src.req(key1, key2) {
            Ok((state, output)) => Ok((state, output)),
            Err(err) => Err((self.f)(err)),
        }
    }
}

impl<K1, K2, K3, S, F, E> DataSrc3Args<K1, K2, K3> for MapErr<S, F>
where
    K1: ?Sized,
    K2: ?Sized,
    K3: ?Sized,
    S: DataSrc3Args<K1, K2, K3>,
    F: Fn(S::Err) -> E + 'static,
{
    type Output = S::Output;
    type Err = E;

    #[inline]
    fn req(
        &self,
        key1: &K1,
        key2: &K2,
        key3: &K3,
    ) -> Result<(NodeStateId, Self::Output), Self::Err> {
        match self.core.src.req(key1, key2, key3) {
            Ok((state, output)) => Ok((state, output)),
            Err(err) => Err((self.f)(err)),
        }
    }
}

impl<K, S, F, E> TakeSnapshot<K> for MapErr<S, F>
where
    K: ?Sized,
    S: TakeSnapshot<K>,
    F: Fn(S::Err) -> E + 'static,
{
    type SnapShot = MapErr<S::SnapShot, F>;
    type SnapShotErr = S::SnapShotErr;

    fn take_snapshot<'a, It>(&self, keys: It) -> Result<Self::SnapShot, Self::SnapShotErr>
    where
        It: IntoIterator<Item = &'a K>,
        K: 'a,
    {
        let snap = self.core.src.take_snapshot(keys)?;
        Ok(MapErr {
            core: Arc::new(_Node {
                src: snap,
                states: StateRecorder::new(Some(64)),
                info: NodeInfo::new(self.core.info.desc()),
            }),
            f: self.f.clone(),
        })
    }
}

impl<K1, K2, S, F, E> TakeSnapshot2Args<K1, K2> for MapErr<S, F>
where
    K1: ?Sized,
    K2: ?Sized,
    S: TakeSnapshot2Args<K1, K2>,
    F: Fn(S::Err) -> E + 'static,
{
    type SnapShot = MapErr<S::SnapShot, F>;
    type SnapShotErr = S::SnapShotErr;

    fn take_snapshot<'a, It>(&self, keys: It) -> Result<Self::SnapShot, Self::SnapShotErr>
    where
        It: IntoIterator<Item = (&'a K1, &'a K2)>,
        K1: 'a,
        K2: 'a,
    {
        let snap = self.core.src.take_snapshot(keys)?;
        Ok(MapErr {
            core: Arc::new(_Node {
                src: snap,
                states: StateRecorder::new(Some(64)),
                info: NodeInfo::new(self.core.info.desc()),
            }),
            f: self.f.clone(),
        })
    }
}

impl<K1, K2, K3, S, F, E> TakeSnapshot3Args<K1, K2, K3> for MapErr<S, F>
where
    K1: ?Sized,
    K2: ?Sized,
    K3: ?Sized,
    S: TakeSnapshot3Args<K1, K2, K3>,
    F: Fn(S::Err) -> E + 'static,
{
    type SnapShot = MapErr<S::SnapShot, F>;
    type SnapShotErr = S::SnapShotErr;

    fn take_snapshot<'a, It>(&self, keys: It) -> Result<Self::SnapShot, Self::SnapShotErr>
    where
        It: IntoIterator<Item = (&'a K1, &'a K2, &'a K3)>,
        K1: 'a,
        K2: 'a,
        K3: 'a,
    {
        let snap = self.core.src.take_snapshot(keys)?;
        Ok(MapErr {
            core: Arc::new(_Node {
                src: snap,
                states: StateRecorder::new(Some(64)),
                info: NodeInfo::new(self.core.info.desc()),
            }),
            f: self.f.clone(),
        })
    }
}

// -----------------------------------------------------------------------------
// Convert
//
#[derive(Debug, Node)]
#[node(transparent = "core")]
pub struct Convert<S, F> {
    core: Arc<_Node<S>>,
    f: Arc<F>,
}

//
// construction
//
impl<S, F> Clone for Convert<S, F> {
    #[inline]
    fn clone(&self) -> Self {
        Self {
            core: self.core.clone(),
            f: self.f.clone(),
        }
    }
}

impl<S: Node, F: 'static> Convert<S, F> {
    pub fn new(desc: impl Into<String>, src: S, f: F) -> Self {
        let info = NodeInfo::new(desc);
        let states = StateRecorder::new(Some(64));
        let core = Arc::new(_Node { src, states, info });
        let subs = Arc::downgrade(&core);
        core.src.accept_subscriber(subs);
        Self {
            core,
            f: Arc::new(f),
        }
    }
}

//
// methods
//
impl<S, F> Convert<S, F> {
    pub fn downstream(&self) -> &S {
        &self.core.src
    }
}

impl<K, S, F, O, E> DataSrc<K> for Convert<S, F>
where
    K: ?Sized,
    S: DataSrc<K>,
    F: Fn(Result<S::Output, S::Err>) -> Result<O, E> + 'static,
{
    type Output = O;
    type Err = E;

    #[inline]
    fn req(&self, key: &K) -> Result<(NodeStateId, Self::Output), Self::Err> {
        match self.core.src.req(key) {
            Ok((_, output)) => (self.f)(Ok(output)).map(|output| (self.core.info.state(), output)),
            Err(err) => (self.f)(Err(err)).map(|o| (self.core.info.state(), o)),
        }
    }
}

impl<K1, K2, S, F, O, E> DataSrc2Args<K1, K2> for Convert<S, F>
where
    K1: ?Sized,
    K2: ?Sized,
    S: DataSrc2Args<K1, K2>,
    F: Fn(Result<S::Output, S::Err>) -> Result<O, E> + 'static,
{
    type Output = O;
    type Err = E;

    #[inline]
    fn req(&self, key1: &K1, key2: &K2) -> Result<(NodeStateId, Self::Output), Self::Err> {
        match self.core.src.req(key1, key2) {
            Ok((_, output)) => (self.f)(Ok(output)).map(|output| (self.core.info.state(), output)),
            Err(err) => (self.f)(Err(err)).map(|o| (self.core.info.state(), o)),
        }
    }
}

impl<K1, K2, K3, S, F, O, E> DataSrc3Args<K1, K2, K3> for Convert<S, F>
where
    K1: ?Sized,
    K2: ?Sized,
    K3: ?Sized,
    S: DataSrc3Args<K1, K2, K3>,
    F: Fn(Result<S::Output, S::Err>) -> Result<O, E> + 'static,
{
    type Output = O;
    type Err = E;

    #[inline]
    fn req(
        &self,
        key1: &K1,
        key2: &K2,
        key3: &K3,
    ) -> Result<(NodeStateId, Self::Output), Self::Err> {
        match self.core.src.req(key1, key2, key3) {
            Ok((_, output)) => (self.f)(Ok(output)).map(|output| (self.core.info.state(), output)),
            Err(err) => (self.f)(Err(err)).map(|o| (self.core.info.state(), o)),
        }
    }
}

impl<K, S, F, O, E> TakeSnapshot<K> for Convert<S, F>
where
    K: ?Sized,
    S: TakeSnapshot<K>,
    F: Fn(Result<S::Output, S::Err>) -> Result<O, E> + 'static,
{
    type SnapShot = Convert<S::SnapShot, F>;
    type SnapShotErr = S::SnapShotErr;

    fn take_snapshot<'a, It>(&self, keys: It) -> Result<Self::SnapShot, Self::SnapShotErr>
    where
        It: IntoIterator<Item = &'a K>,
        K: 'a,
    {
        let snap = self.core.src.take_snapshot(keys)?;
        Ok(Convert {
            core: Arc::new(_Node {
                src: snap,
                states: StateRecorder::new(Some(64)),
                info: NodeInfo::new(self.core.info.desc()),
            }),
            f: self.f.clone(),
        })
    }
}

impl<K1, K2, S, F, O, E> TakeSnapshot2Args<K1, K2> for Convert<S, F>
where
    K1: ?Sized,
    K2: ?Sized,
    S: TakeSnapshot2Args<K1, K2>,
    F: Fn(Result<S::Output, S::Err>) -> Result<O, E> + 'static,
{
    type SnapShot = Convert<S::SnapShot, F>;
    type SnapShotErr = S::SnapShotErr;

    fn take_snapshot<'a, It>(&self, keys: It) -> Result<Self::SnapShot, Self::SnapShotErr>
    where
        It: IntoIterator<Item = (&'a K1, &'a K2)>,
        K1: 'a,
        K2: 'a,
    {
        let snap = self.core.src.take_snapshot(keys)?;
        Ok(Convert {
            core: Arc::new(_Node {
                src: snap,
                states: StateRecorder::new(Some(64)),
                info: NodeInfo::new(self.core.info.desc()),
            }),
            f: self.f.clone(),
        })
    }
}

impl<K1, K2, K3, S, F, O, E> TakeSnapshot3Args<K1, K2, K3> for Convert<S, F>
where
    K1: ?Sized,
    K2: ?Sized,
    K3: ?Sized,
    S: TakeSnapshot3Args<K1, K2, K3>,
    F: Fn(Result<S::Output, S::Err>) -> Result<O, E> + 'static,
{
    type SnapShot = Convert<S::SnapShot, F>;
    type SnapShotErr = S::SnapShotErr;

    fn take_snapshot<'a, It>(&self, keys: It) -> Result<Self::SnapShot, Self::SnapShotErr>
    where
        It: IntoIterator<Item = (&'a K1, &'a K2, &'a K3)>,
        K1: 'a,
        K2: 'a,
        K3: 'a,
    {
        let snap = self.core.src.take_snapshot(keys)?;
        Ok(Convert {
            core: Arc::new(_Node {
                src: snap,
                states: StateRecorder::new(Some(64)),
                info: NodeInfo::new(self.core.info.desc()),
            }),
            f: self.f.clone(),
        })
    }
}

// -----------------------------------------------------------------------------
// WithLogger
//
#[derive(Debug, Node)]
#[node(transparent = "core")]
pub struct WithLogger<S, F> {
    core: Arc<_Node<S>>,
    logger: Arc<F>,
}

//
// construction
//
impl<S, F> Clone for WithLogger<S, F> {
    #[inline]
    fn clone(&self) -> Self {
        Self {
            core: self.core.clone(),
            logger: self.logger.clone(),
        }
    }
}

impl<S: Node, F: 'static> WithLogger<S, F> {
    pub fn new(desc: impl Into<String>, src: S, logger: F) -> Self {
        let info = NodeInfo::new(desc);
        let states = StateRecorder::new(Some(64));
        let core = Arc::new(_Node { src, states, info });
        let subs = Arc::downgrade(&core);
        core.src.accept_subscriber(subs);
        Self {
            core,
            logger: Arc::new(logger),
        }
    }
}

//
// methods
//
impl<S, F> WithLogger<S, F> {
    pub fn downstream(&self) -> &S {
        &self.core.src
    }
}

impl<K, S, F> DataSrc<K> for WithLogger<S, F>
where
    K: ?Sized,
    S: DataSrc<K>,
    F: Fn(&K, &Result<(NodeStateId, S::Output), S::Err>) + 'static,
{
    type Output = S::Output;
    type Err = S::Err;

    #[inline]
    fn req(&self, key: &K) -> Result<(NodeStateId, Self::Output), Self::Err> {
        let result = self.core.src.req(key);
        (self.logger)(key, &result);
        result
    }
}

impl<K1, K2, S, F> DataSrc2Args<K1, K2> for WithLogger<S, F>
where
    K1: ?Sized,
    K2: ?Sized,
    S: DataSrc2Args<K1, K2>,
    F: Fn(&K1, &K2, &Result<(NodeStateId, S::Output), S::Err>) + 'static,
{
    type Output = S::Output;
    type Err = S::Err;

    #[inline]
    fn req(&self, key1: &K1, key2: &K2) -> Result<(NodeStateId, Self::Output), Self::Err> {
        let result = self.core.src.req(key1, key2);
        (self.logger)(key1, key2, &result);
        result
    }
}

impl<K1, K2, K3, S, F> DataSrc3Args<K1, K2, K3> for WithLogger<S, F>
where
    K1: ?Sized,
    K2: ?Sized,
    K3: ?Sized,
    S: DataSrc3Args<K1, K2, K3>,
    F: Fn(&K1, &K2, &K3, &Result<(NodeStateId, S::Output), S::Err>) + 'static,
{
    type Output = S::Output;
    type Err = S::Err;

    #[inline]
    fn req(
        &self,
        key1: &K1,
        key2: &K2,
        key3: &K3,
    ) -> Result<(NodeStateId, Self::Output), Self::Err> {
        let result = self.core.src.req(key1, key2, key3);
        (self.logger)(key1, key2, key3, &result);
        result
    }
}

impl<K, S, F> TakeSnapshot<K> for WithLogger<S, F>
where
    K: ?Sized,
    S: TakeSnapshot<K>,
    F: Fn(&K, &Result<(NodeStateId, S::Output), S::Err>) + 'static,
{
    type SnapShot = WithLogger<S::SnapShot, F>;
    type SnapShotErr = S::SnapShotErr;

    fn take_snapshot<'a, It>(&self, keys: It) -> Result<Self::SnapShot, Self::SnapShotErr>
    where
        It: IntoIterator<Item = &'a K>,
        K: 'a,
    {
        let snap = self.core.src.take_snapshot(keys)?;
        Ok(WithLogger {
            core: Arc::new(_Node {
                src: snap,
                states: StateRecorder::new(Some(64)),
                info: NodeInfo::new(self.core.info.desc()),
            }),
            logger: self.logger.clone(),
        })
    }
}

impl<K1, K2, S, F> TakeSnapshot2Args<K1, K2> for WithLogger<S, F>
where
    K1: ?Sized,
    K2: ?Sized,
    S: TakeSnapshot2Args<K1, K2>,
    F: Fn(&K1, &K2, &Result<(NodeStateId, S::Output), S::Err>) + 'static,
{
    type SnapShot = WithLogger<S::SnapShot, F>;
    type SnapShotErr = S::SnapShotErr;

    fn take_snapshot<'a, It>(&self, keys: It) -> Result<Self::SnapShot, Self::SnapShotErr>
    where
        It: IntoIterator<Item = (&'a K1, &'a K2)>,
        K1: 'a,
        K2: 'a,
    {
        let snap = self.core.src.take_snapshot(keys)?;
        Ok(WithLogger {
            core: Arc::new(_Node {
                src: snap,
                states: StateRecorder::new(Some(64)),
                info: NodeInfo::new(self.core.info.desc()),
            }),
            logger: self.logger.clone(),
        })
    }
}

impl<K1, K2, K3, S, F> TakeSnapshot3Args<K1, K2, K3> for WithLogger<S, F>
where
    K1: ?Sized,
    K2: ?Sized,
    K3: ?Sized,
    S: TakeSnapshot3Args<K1, K2, K3>,
    F: Fn(&K1, &K2, &K3, &Result<(NodeStateId, S::Output), S::Err>) + 'static,
{
    type SnapShot = WithLogger<S::SnapShot, F>;
    type SnapShotErr = S::SnapShotErr;

    fn take_snapshot<'a, It>(&self, keys: It) -> Result<Self::SnapShot, Self::SnapShotErr>
    where
        It: IntoIterator<Item = (&'a K1, &'a K2, &'a K3)>,
        K1: 'a,
        K2: 'a,
        K3: 'a,
    {
        let snap = self.core.src.take_snapshot(keys)?;
        Ok(WithLogger {
            core: Arc::new(_Node {
                src: snap,
                states: StateRecorder::new(Some(64)),
                info: NodeInfo::new(self.core.info.desc()),
            }),
            logger: self.logger.clone(),
        })
    }
}

// =============================================================================
#[cfg(test)]
mod tests {
    use std::sync::Mutex;

    use maplit::hashmap;
    use rstest::{fixture, rstest};

    use crate::datasrc::{
        ImmutableOnMemorySrc, ImmutableOnMemorySrc2Args, ImmutableOnMemorySrc3Args,
    };

    use super::*;

    #[fixture]
    fn src_1arg() -> ImmutableOnMemorySrc<String, i32> {
        let src = ImmutableOnMemorySrc::with_data(
            "src",
            hashmap! {
                "a".to_owned() => 1,
                "b".to_owned() => 2,
                "c".to_owned() => 3,
            },
        );
        src
    }

    #[fixture]
    fn src_2args() -> ImmutableOnMemorySrc2Args<String, String, i32> {
        let src = ImmutableOnMemorySrc2Args::with_data(
            "src",
            hashmap! {
                ("a".to_owned(), "x".to_owned()) => 1,
                ("a".to_owned(), "y".to_owned()) => 2,
                ("b".to_owned(), "x".to_owned()) => 3,
                ("b".to_owned(), "y".to_owned()) => 4,
                ("c".to_owned(), "x".to_owned()) => 5,
                ("c".to_owned(), "y".to_owned()) => 6,
            },
        );
        src
    }

    #[fixture]
    fn src_3args() -> ImmutableOnMemorySrc3Args<String, String, String, i32> {
        let src = ImmutableOnMemorySrc3Args::with_data(
            "src",
            hashmap! {
                ("a".to_owned(), "x".to_owned(), "i".to_owned()) => 1,
                ("a".to_owned(), "x".to_owned(), "j".to_owned()) => 2,
                ("a".to_owned(), "y".to_owned(), "i".to_owned()) => 3,
                ("a".to_owned(), "y".to_owned(), "j".to_owned()) => 4,
                ("b".to_owned(), "x".to_owned(), "i".to_owned()) => 5,
                ("b".to_owned(), "x".to_owned(), "j".to_owned()) => 6,
                ("b".to_owned(), "y".to_owned(), "i".to_owned()) => 7,
                ("b".to_owned(), "y".to_owned(), "j".to_owned()) => 8,
                ("c".to_owned(), "x".to_owned(), "i".to_owned()) => 9,
                ("c".to_owned(), "x".to_owned(), "j".to_owned()) => 10,
                ("c".to_owned(), "y".to_owned(), "i".to_owned()) => 11,
                ("c".to_owned(), "y".to_owned(), "j".to_owned()) => 12,
            },
        );
        src
    }

    #[rstest]
    fn test_map_1arg(src_1arg: ImmutableOnMemorySrc<String, i32>) {
        let src = Map::new("map", src_1arg, |x| x * 2);

        // ok
        assert_eq!(src.req("a").unwrap().1, 2);
        assert_eq!(src.req("b").unwrap().1, 4);
        assert_eq!(src.req("c").unwrap().1, 6);

        // err
        assert!(src.req("d").is_err());
    }

    #[rstest]
    fn test_map_2args(src_2args: ImmutableOnMemorySrc2Args<String, String, i32>) {
        let src = Map::new("map", src_2args, |x| x * 2);

        // ok
        assert_eq!(src.req("a", "x").unwrap().1, 2);
        assert_eq!(src.req("a", "y").unwrap().1, 4);
        assert_eq!(src.req("b", "x").unwrap().1, 6);
        assert_eq!(src.req("b", "y").unwrap().1, 8);
        assert_eq!(src.req("c", "x").unwrap().1, 10);
        assert_eq!(src.req("c", "y").unwrap().1, 12);

        // err
        assert!(src.req("d", "x").is_err());
        assert!(src.req("a", "z").is_err());
    }

    #[rstest]
    fn test_map_3args(src_3args: ImmutableOnMemorySrc3Args<String, String, String, i32>) {
        let src = Map::new("map", src_3args, |x| x * 2);

        // ok
        assert_eq!(src.req("a", "x", "i").unwrap().1, 2);
        assert_eq!(src.req("a", "x", "j").unwrap().1, 4);
        assert_eq!(src.req("a", "y", "i").unwrap().1, 6);
        assert_eq!(src.req("a", "y", "j").unwrap().1, 8);
        assert_eq!(src.req("b", "x", "i").unwrap().1, 10);
        assert_eq!(src.req("b", "x", "j").unwrap().1, 12);
        assert_eq!(src.req("b", "y", "i").unwrap().1, 14);
        assert_eq!(src.req("b", "y", "j").unwrap().1, 16);
        assert_eq!(src.req("c", "x", "i").unwrap().1, 18);
        assert_eq!(src.req("c", "x", "j").unwrap().1, 20);
        assert_eq!(src.req("c", "y", "i").unwrap().1, 22);
        assert_eq!(src.req("c", "y", "j").unwrap().1, 24);

        // err
        assert!(src.req("d", "x", "i").is_err());
        assert!(src.req("a", "z", "i").is_err());
        assert!(src.req("a", "x", "k").is_err());
    }

    #[rstest]
    fn test_map_err_1arg(src_1arg: ImmutableOnMemorySrc<String, i32>) {
        let src = MapErr::new("map", src_1arg, |_| "error".to_owned());

        // ok
        assert_eq!(src.req("a").unwrap().1, 1);
        assert_eq!(src.req("b").unwrap().1, 2);
        assert_eq!(src.req("c").unwrap().1, 3);

        // err
        assert!(src.req("d").is_err());
        assert!(src.req("d").unwrap_err() == "error");
    }

    #[rstest]
    fn test_map_err_2args(src_2args: ImmutableOnMemorySrc2Args<String, String, i32>) {
        let src = MapErr::new("map", src_2args, |_| "error".to_owned());

        // ok
        assert_eq!(src.req("a", "x").unwrap().1, 1);
        assert_eq!(src.req("a", "y").unwrap().1, 2);
        assert_eq!(src.req("b", "x").unwrap().1, 3);
        assert_eq!(src.req("b", "y").unwrap().1, 4);
        assert_eq!(src.req("c", "x").unwrap().1, 5);
        assert_eq!(src.req("c", "y").unwrap().1, 6);

        // err
        assert!(src.req("d", "x").is_err());
        assert!(src.req("d", "x").unwrap_err() == "error");
        assert!(src.req("a", "z").is_err());
        assert!(src.req("a", "z").unwrap_err() == "error");
    }

    #[rstest]
    fn test_map_err_3args(src_3args: ImmutableOnMemorySrc3Args<String, String, String, i32>) {
        let src = MapErr::new("map", src_3args, |_| "error".to_owned());

        // ok
        assert_eq!(src.req("a", "x", "i").unwrap().1, 1);
        assert_eq!(src.req("a", "x", "j").unwrap().1, 2);
        assert_eq!(src.req("a", "y", "i").unwrap().1, 3);
        assert_eq!(src.req("a", "y", "j").unwrap().1, 4);
        assert_eq!(src.req("b", "x", "i").unwrap().1, 5);
        assert_eq!(src.req("b", "x", "j").unwrap().1, 6);
        assert_eq!(src.req("b", "y", "i").unwrap().1, 7);
        assert_eq!(src.req("b", "y", "j").unwrap().1, 8);
        assert_eq!(src.req("c", "x", "i").unwrap().1, 9);
        assert_eq!(src.req("c", "x", "j").unwrap().1, 10);
        assert_eq!(src.req("c", "y", "i").unwrap().1, 11);
        assert_eq!(src.req("c", "y", "j").unwrap().1, 12);

        // err
        assert!(src.req("d", "x", "i").is_err());
        assert!(src.req("d", "x", "i").unwrap_err() == "error");
        assert!(src.req("a", "z", "i").is_err());
        assert!(src.req("a", "z", "i").unwrap_err() == "error");
        assert!(src.req("a", "x", "k").is_err());
        assert!(src.req("a", "x", "k").unwrap_err() == "error");
    }

    #[rstest]
    fn test_convert_1arg(src_1arg: ImmutableOnMemorySrc<String, i32>) {
        let src = Convert::new("convert", src_1arg, |r| match r {
            Ok(x) => {
                if x % 2 == 0 {
                    Ok(x)
                } else {
                    Err("error".to_owned())
                }
            }
            Err(_) => Err("downstream error".to_owned()),
        });

        // ok
        assert_eq!(src.req("b").unwrap().1, 2);

        // err
        assert!(src.req("a").is_err());
        assert!(src.req("a").unwrap_err() == "error");

        assert!(src.req("c").is_err());
        assert!(src.req("c").unwrap_err() == "error");

        assert!(src.req("d").is_err());
        assert!(src.req("d").unwrap_err() == "downstream error");
    }

    #[rstest]
    fn test_convert_2args(src_2args: ImmutableOnMemorySrc2Args<String, String, i32>) {
        let src = Convert::new("convert", src_2args, |r| match r {
            Ok(x) => {
                if x % 2 == 0 {
                    Ok(x)
                } else {
                    Err("error".to_owned())
                }
            }
            Err(_) => Err("downstream error".to_owned()),
        });

        // ok
        assert_eq!(src.req("a", "y").unwrap().1, 2);
        assert_eq!(src.req("b", "y").unwrap().1, 4);
        assert_eq!(src.req("c", "y").unwrap().1, 6);

        // err
        assert!(src.req("a", "x").is_err());
        assert!(src.req("a", "x").unwrap_err() == "error");

        assert!(src.req("d", "x").is_err());
        assert!(src.req("d", "x").unwrap_err() == "downstream error");
    }

    #[rstest]
    fn test_convert_3args(src_3args: ImmutableOnMemorySrc3Args<String, String, String, i32>) {
        let src = Convert::new("convert", src_3args, |r| match r {
            Ok(x) => {
                if x % 2 == 0 {
                    Ok(x)
                } else {
                    Err("error".to_owned())
                }
            }
            Err(_) => Err("downstream error".to_owned()),
        });

        // ok
        assert_eq!(src.req("a", "x", "j").unwrap().1, 2);
        assert_eq!(src.req("b", "x", "j").unwrap().1, 6);
        assert_eq!(src.req("c", "x", "j").unwrap().1, 10);

        // err
        assert!(src.req("a", "x", "i").is_err());
        assert!(src.req("a", "x", "i").unwrap_err() == "error");

        assert!(src.req("a", "y", "i").is_err());
        assert!(src.req("a", "y", "i").unwrap_err() == "error");

        assert!(src.req("d", "x", "i").is_err());
        assert!(src.req("d", "x", "i").unwrap_err() == "downstream error");
    }

    #[rstest]
    fn test_with_logger_1arg(src_1arg: ImmutableOnMemorySrc<String, i32>) {
        let (reader, logger) = {
            let msg = Arc::new(Mutex::new(String::new()));
            let reader = msg.clone();
            let logger = move |k: &str, r: &Result<(NodeStateId, i32), anyhow::Error>| {
                let mut msg = msg.lock().unwrap();
                match r {
                    Ok((_, v)) => *msg = format!("[ok] {}: {}", k, v),
                    Err(_) => *msg = format!("[ng] {}", k),
                }
            };
            (reader, logger)
        };
        let src = WithLogger::new("with_logger", src_1arg, logger);

        // ok
        assert_eq!(src.req("a").unwrap().1, 1);
        assert_eq!(reader.lock().unwrap().as_str(), "[ok] a: 1");
        assert_eq!(src.req("b").unwrap().1, 2);
        assert_eq!(reader.lock().unwrap().as_str(), "[ok] b: 2");
        assert_eq!(src.req("c").unwrap().1, 3);
        assert_eq!(reader.lock().unwrap().as_str(), "[ok] c: 3");

        // err
        assert!(src.req("d").is_err());
        assert_eq!(reader.lock().unwrap().as_str(), "[ng] d");
    }

    #[rstest]
    fn test_with_logger_2args(src_2args: ImmutableOnMemorySrc2Args<String, String, i32>) {
        let (reader, logger) = {
            let msg = Arc::new(Mutex::new(String::new()));
            let reader = msg.clone();
            let logger =
                move |k1: &str, k2: &str, r: &Result<(NodeStateId, i32), anyhow::Error>| {
                    let mut msg = msg.lock().unwrap();
                    match r {
                        Ok((_, v)) => *msg = format!("[ok] ({}, {}): {}", k1, k2, v),
                        Err(_) => *msg = format!("[ng] ({}, {})", k1, k2),
                    }
                };
            (reader, logger)
        };
        let src = WithLogger::new("with_logger", src_2args, logger);

        // ok
        assert_eq!(src.req("a", "x").unwrap().1, 1);
        assert_eq!(reader.lock().unwrap().as_str(), "[ok] (a, x): 1");
        assert_eq!(src.req("b", "x").unwrap().1, 3);
        assert_eq!(reader.lock().unwrap().as_str(), "[ok] (b, x): 3");
        assert_eq!(src.req("c", "x").unwrap().1, 5);
        assert_eq!(reader.lock().unwrap().as_str(), "[ok] (c, x): 5");

        // err
        assert!(src.req("d", "x").is_err());
        assert_eq!(reader.lock().unwrap().as_str(), "[ng] (d, x)");
    }

    #[rstest]
    fn test_with_logger_3args(src_3args: ImmutableOnMemorySrc3Args<String, String, String, i32>) {
        let (reader, logger) = {
            let msg = Arc::new(Mutex::new(String::new()));
            let reader = msg.clone();
            let logger =
                move |k1: &str,
                      k2: &str,
                      k3: &str,
                      r: &Result<(NodeStateId, i32), anyhow::Error>| {
                    let mut msg = msg.lock().unwrap();
                    match r {
                        Ok((_, v)) => *msg = format!("[ok] ({}, {}, {}): {}", k1, k2, k3, v),
                        Err(_) => *msg = format!("[ng] ({}, {}, {})", k1, k2, k3),
                    }
                };
            (reader, logger)
        };
        let src = WithLogger::new("with_logger", src_3args, logger);

        // ok
        assert_eq!(src.req("a", "x", "i").unwrap().1, 1);
        assert_eq!(reader.lock().unwrap().as_str(), "[ok] (a, x, i): 1");
        assert_eq!(src.req("b", "x", "i").unwrap().1, 5);
        assert_eq!(reader.lock().unwrap().as_str(), "[ok] (b, x, i): 5");
        assert_eq!(src.req("c", "x", "i").unwrap().1, 9);
        assert_eq!(reader.lock().unwrap().as_str(), "[ok] (c, x, i): 9");

        // err
        assert!(src.req("d", "x", "i").is_err());
        assert_eq!(reader.lock().unwrap().as_str(), "[ng] (d, x, i)");
    }
}
