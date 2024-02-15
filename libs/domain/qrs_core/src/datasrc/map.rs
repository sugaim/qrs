// use std::sync::Arc;

use std::sync::{Arc, Mutex, Weak};

use maplit::btreeset;
use qrs_core_derive::{Listener, Node};

use super::{
    DataSrc, Listener, NodeId, Notifier, StateId, Tree, _private::_UnaryPassThroughNode,
    node::DataSrc2Args, snapshot::TakeSnapshot3Args, DataSrc3Args, Node, TakeSnapshot,
    TakeSnapshot2Args,
};

// -----------------------------------------------------------------------------
// Map
//
#[derive(Debug, Node, Listener)]
#[node(transparent = "node")]
#[listener(transparent = "node")]
pub struct Map<S, F> {
    node: Arc<Mutex<_UnaryPassThroughNode>>,
    src: S,
    f: F,
}

//
// construction
//
impl<S: Notifier, F> Map<S, F> {
    #[inline]
    pub fn new(desc: impl Into<String>, mut src: S, f: F) -> Self {
        let node = _UnaryPassThroughNode::new_and_reg(desc, &mut src);
        Self { node, f, src }
    }
}
impl<S: Notifier + Clone, F: Clone> Clone for Map<S, F> {
    #[inline]
    fn clone(&self) -> Self {
        Self::new(
            self.node.lock().unwrap().desc(),
            self.src.clone(),
            self.f.clone(),
        )
    }
}

//
// methods
//
impl<S, F> Map<S, F> {
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

impl<S, F> Notifier for Map<S, F>
where
    S: Notifier,
    F: 'static + Send + Sync,
{
    #[inline]
    fn state(&self) -> StateId {
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
    fn accept_listener(&mut self, subsc: Weak<Mutex<dyn Listener>>) {
        self.node.lock().unwrap().accept_subscriber(subsc);
    }

    #[inline]
    fn remove_listener(&mut self, id: &NodeId) {
        self.node.lock().unwrap().remove_subscriber(id);
    }
}

impl<S, F, O> DataSrc for Map<S, F>
where
    S: DataSrc,
    F: 'static + Send + Sync + Fn(S::Output) -> O,
{
    type Key = S::Key;
    type Output = O;
    type Err = S::Err;

    #[inline]
    fn req(&self, key: &S::Key) -> Result<Self::Output, Self::Err> {
        self.src.req(key).map(|o| (self.f)(o))
    }
}

impl<S, F, O> DataSrc2Args for Map<S, F>
where
    S: DataSrc2Args,
    F: 'static + Send + Sync + Fn(S::Output) -> O,
{
    type Key1 = S::Key1;
    type Key2 = S::Key2;
    type Output = O;
    type Err = S::Err;

    #[inline]
    fn req(&self, key1: &Self::Key1, key2: &Self::Key2) -> Result<Self::Output, Self::Err> {
        self.src.req(key1, key2).map(|o| (self.f)(o))
    }
}

impl<S, F, O> DataSrc3Args for Map<S, F>
where
    S: DataSrc3Args,
    F: 'static + Send + Sync + Fn(S::Output) -> O,
{
    type Key1 = S::Key1;
    type Key2 = S::Key2;
    type Key3 = S::Key3;
    type Output = O;
    type Err = S::Err;

    #[inline]
    fn req(
        &self,
        key1: &Self::Key1,
        key2: &Self::Key2,
        key3: &Self::Key3,
    ) -> Result<Self::Output, Self::Err> {
        self.src.req(key1, key2, key3).map(|o| (self.f)(o))
    }
}

impl<S, F, O> TakeSnapshot for Map<S, F>
where
    S: TakeSnapshot,
    F: 'static + Send + Sync + Clone + Fn(S::Output) -> O,
{
    type SnapShot = Map<S::SnapShot, F>;
    type SnapShotErr = S::SnapShotErr;

    #[inline]
    fn take_snapshot<'a, It>(&self, keys: It) -> Result<Self::SnapShot, Self::SnapShotErr>
    where
        It: IntoIterator<Item = &'a Self::Key>,
        Self::Key: 'a,
    {
        self.src
            .take_snapshot(keys)
            .map(|snap| Map::new(self.node.lock().unwrap().desc(), snap, self.f.clone()))
    }
}

impl<S, F, O> TakeSnapshot2Args for Map<S, F>
where
    S: TakeSnapshot2Args,
    F: 'static + Send + Sync + Clone + Fn(S::Output) -> O,
{
    type SnapShot = Map<S::SnapShot, F>;
    type SnapShotErr = S::SnapShotErr;

    #[inline]
    fn take_snapshot<'a, It>(&self, keys: It) -> Result<Self::SnapShot, Self::SnapShotErr>
    where
        It: IntoIterator<Item = (&'a Self::Key1, &'a Self::Key2)>,
        Self::Key1: 'a,
        Self::Key2: 'a,
    {
        self.src
            .take_snapshot(keys)
            .map(|snap| Map::new(self.node.lock().unwrap().desc(), snap, self.f.clone()))
    }
}

impl<S, F, O> TakeSnapshot3Args for Map<S, F>
where
    S: TakeSnapshot3Args,
    F: 'static + Send + Sync + Clone + Fn(S::Output) -> O,
{
    type SnapShot = Map<S::SnapShot, F>;
    type SnapShotErr = S::SnapShotErr;

    #[inline]
    fn take_snapshot<'a, It>(&self, keys: It) -> Result<Self::SnapShot, Self::SnapShotErr>
    where
        It: IntoIterator<Item = (&'a Self::Key1, &'a Self::Key2, &'a Self::Key3)>,
        Self::Key1: 'a,
        Self::Key2: 'a,
        Self::Key3: 'a,
    {
        self.src
            .take_snapshot(keys)
            .map(|snap| Map::new(self.node.lock().unwrap().desc(), snap, self.f.clone()))
    }
}

// -----------------------------------------------------------------------------
// MapErr
//
#[derive(Debug, Node, Listener)]
#[node(transparent = "node")]
#[listener(transparent = "node")]
pub struct MapErr<S, F> {
    node: Arc<Mutex<_UnaryPassThroughNode>>,
    src: S,
    f: F,
}

//
// construction
//
impl<S: Notifier, F> MapErr<S, F> {
    #[inline]
    pub fn new(desc: impl Into<String>, mut src: S, f: F) -> Self {
        let node = _UnaryPassThroughNode::new_and_reg(desc, &mut src);
        Self { node, f, src }
    }
}

impl<S: Clone + Notifier, F: Clone> Clone for MapErr<S, F> {
    #[inline]
    fn clone(&self) -> Self {
        Self::new(
            self.node.lock().unwrap().desc(),
            self.src.clone(),
            self.f.clone(),
        )
    }
}

//
// methods
//
impl<S, F> MapErr<S, F> {
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

impl<S, F> Notifier for MapErr<S, F>
where
    S: Notifier,
    F: 'static + Send + Sync,
{
    #[inline]
    fn state(&self) -> StateId {
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
    fn accept_listener(&mut self, subsc: Weak<Mutex<dyn Listener>>) {
        self.node.lock().unwrap().accept_subscriber(subsc);
    }

    #[inline]
    fn remove_listener(&mut self, id: &NodeId) {
        self.node.lock().unwrap().remove_subscriber(id);
    }
}

impl<S, F, E> DataSrc for MapErr<S, F>
where
    S: DataSrc,
    F: 'static + Send + Sync + Fn(S::Err) -> E,
{
    type Key = S::Key;
    type Output = S::Output;
    type Err = E;

    #[inline]
    fn req(&self, key: &Self::Key) -> Result<Self::Output, Self::Err> {
        self.src.req(key).map_err(|err| (self.f)(err))
    }
}

impl<S, F, E> DataSrc2Args for MapErr<S, F>
where
    S: DataSrc2Args,
    F: 'static + Send + Sync + Fn(S::Err) -> E,
{
    type Key1 = S::Key1;
    type Key2 = S::Key2;
    type Output = S::Output;
    type Err = E;

    #[inline]
    fn req(&self, key1: &Self::Key1, key2: &Self::Key2) -> Result<Self::Output, Self::Err> {
        self.src.req(key1, key2).map_err(|err| (self.f)(err))
    }
}

impl<S, F, E> DataSrc3Args for MapErr<S, F>
where
    S: DataSrc3Args,
    F: 'static + Send + Sync + Fn(S::Err) -> E,
{
    type Key1 = S::Key1;
    type Key2 = S::Key2;
    type Key3 = S::Key3;
    type Output = S::Output;
    type Err = E;

    #[inline]
    fn req(
        &self,
        key1: &Self::Key1,
        key2: &Self::Key2,
        key3: &Self::Key3,
    ) -> Result<Self::Output, Self::Err> {
        self.src.req(key1, key2, key3).map_err(|err| (self.f)(err))
    }
}

impl<S, F, E> TakeSnapshot for MapErr<S, F>
where
    S: TakeSnapshot,
    F: 'static + Send + Sync + Clone + Fn(S::Err) -> E,
{
    type SnapShot = MapErr<S::SnapShot, F>;
    type SnapShotErr = S::SnapShotErr;

    #[inline]
    fn take_snapshot<'a, It>(&self, keys: It) -> Result<Self::SnapShot, Self::SnapShotErr>
    where
        It: IntoIterator<Item = &'a Self::Key>,
        Self::Key: 'a,
    {
        self.src
            .take_snapshot(keys)
            .map(|snap| MapErr::new(self.node.lock().unwrap().desc(), snap, self.f.clone()))
    }
}

impl<S, F, E> TakeSnapshot2Args for MapErr<S, F>
where
    S: TakeSnapshot2Args,
    F: 'static + Send + Sync + Clone + Fn(S::Err) -> E,
{
    type SnapShot = MapErr<S::SnapShot, F>;
    type SnapShotErr = S::SnapShotErr;

    #[inline]
    fn take_snapshot<'a, It>(&self, keys: It) -> Result<Self::SnapShot, Self::SnapShotErr>
    where
        It: IntoIterator<Item = (&'a Self::Key1, &'a Self::Key2)>,
        Self::Key1: 'a,
        Self::Key2: 'a,
    {
        self.src
            .take_snapshot(keys)
            .map(|snap| MapErr::new(self.node.lock().unwrap().desc(), snap, self.f.clone()))
    }
}

impl<S, F, E> TakeSnapshot3Args for MapErr<S, F>
where
    S: TakeSnapshot3Args,
    F: 'static + Send + Sync + Clone + Fn(S::Err) -> E,
{
    type SnapShot = MapErr<S::SnapShot, F>;
    type SnapShotErr = S::SnapShotErr;

    #[inline]
    fn take_snapshot<'a, It>(&self, keys: It) -> Result<Self::SnapShot, Self::SnapShotErr>
    where
        It: IntoIterator<Item = (&'a Self::Key1, &'a Self::Key2, &'a Self::Key3)>,
        Self::Key1: 'a,
        Self::Key2: 'a,
        Self::Key3: 'a,
    {
        self.src
            .take_snapshot(keys)
            .map(|snap| MapErr::new(self.node.lock().unwrap().desc(), snap, self.f.clone()))
    }
}

// -----------------------------------------------------------------------------
// Convert
//
#[derive(Debug, Node, Listener)]
#[node(transparent = "node")]
#[listener(transparent = "node")]
pub struct Convert<S, F> {
    node: Arc<Mutex<_UnaryPassThroughNode>>,
    src: S,
    f: F,
}

//
// construction
//
impl<S: Notifier, F> Convert<S, F> {
    #[inline]
    pub fn new(desc: impl Into<String>, mut src: S, f: F) -> Self {
        let node = _UnaryPassThroughNode::new_and_reg(desc, &mut src);
        Self { node, f, src }
    }
}

impl<S: Clone + Notifier, F: Clone> Clone for Convert<S, F> {
    #[inline]
    fn clone(&self) -> Self {
        Self::new(
            self.node.lock().unwrap().desc(),
            self.src.clone(),
            self.f.clone(),
        )
    }
}

//
// methods
//
impl<S, F> Convert<S, F> {
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

impl<S: Notifier, F: 'static + Send + Sync> Notifier for Convert<S, F> {
    #[inline]
    fn state(&self) -> StateId {
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
    fn accept_listener(&mut self, subsc: Weak<Mutex<dyn Listener>>) {
        self.node.lock().unwrap().accept_subscriber(subsc);
    }

    #[inline]
    fn remove_listener(&mut self, id: &NodeId) {
        self.node.lock().unwrap().remove_subscriber(id);
    }
}

impl<S, F, O, E> DataSrc for Convert<S, F>
where
    S: DataSrc,
    F: 'static + Send + Sync + Fn(&S::Key, Result<S::Output, S::Err>) -> Result<O, E>,
{
    type Key = S::Key;
    type Output = O;
    type Err = E;

    #[inline]
    fn req(&self, key: &Self::Key) -> Result<Self::Output, Self::Err> {
        (self.f)(key, self.src.req(key))
    }
}

impl<S, F, O, E> DataSrc2Args for Convert<S, F>
where
    S: DataSrc2Args,
    F: 'static + Send + Sync + Fn(&S::Key1, &S::Key2, Result<S::Output, S::Err>) -> Result<O, E>,
{
    type Key1 = S::Key1;
    type Key2 = S::Key2;
    type Output = O;
    type Err = E;

    #[inline]
    fn req(&self, key1: &Self::Key1, key2: &Self::Key2) -> Result<Self::Output, Self::Err> {
        (self.f)(key1, key2, self.src.req(key1, key2))
    }
}

impl<S, F, O, E> DataSrc3Args for Convert<S, F>
where
    S: DataSrc3Args,
    F: 'static
        + Send
        + Sync
        + Fn(&S::Key1, &S::Key2, &S::Key3, Result<S::Output, S::Err>) -> Result<O, E>,
{
    type Key1 = S::Key1;
    type Key2 = S::Key2;
    type Key3 = S::Key3;
    type Output = O;
    type Err = E;

    #[inline]
    fn req(
        &self,
        key1: &Self::Key1,
        key2: &Self::Key2,
        key3: &Self::Key3,
    ) -> Result<Self::Output, Self::Err> {
        (self.f)(key1, key2, key3, self.src.req(key1, key2, key3))
    }
}

impl<S, F, O, E> TakeSnapshot for Convert<S, F>
where
    S: TakeSnapshot,
    F: 'static + Send + Sync + Clone + Fn(&S::Key, Result<S::Output, S::Err>) -> Result<O, E>,
{
    type SnapShot = Convert<S::SnapShot, F>;
    type SnapShotErr = S::SnapShotErr;

    #[inline]
    fn take_snapshot<'a, It>(&self, keys: It) -> Result<Self::SnapShot, Self::SnapShotErr>
    where
        It: IntoIterator<Item = &'a Self::Key>,
        Self::Key: 'a,
    {
        self.src
            .take_snapshot(keys)
            .map(|snap| Convert::new(self.node.lock().unwrap().desc(), snap, self.f.clone()))
    }
}

impl<S, F, O, E> TakeSnapshot2Args for Convert<S, F>
where
    S: TakeSnapshot2Args,
    F: 'static
        + Send
        + Sync
        + Clone
        + Fn(&S::Key1, &S::Key2, Result<S::Output, S::Err>) -> Result<O, E>,
{
    type SnapShot = Convert<S::SnapShot, F>;
    type SnapShotErr = S::SnapShotErr;

    #[inline]
    fn take_snapshot<'a, It>(&self, keys: It) -> Result<Self::SnapShot, Self::SnapShotErr>
    where
        It: IntoIterator<Item = (&'a Self::Key1, &'a Self::Key2)>,
        Self::Key1: 'a,
        Self::Key2: 'a,
    {
        self.src
            .take_snapshot(keys)
            .map(|snap| Convert::new(self.node.lock().unwrap().desc(), snap, self.f.clone()))
    }
}

impl<S, F, O, E> TakeSnapshot3Args for Convert<S, F>
where
    S: TakeSnapshot3Args,
    F: 'static
        + Send
        + Sync
        + Clone
        + Fn(&S::Key1, &S::Key2, &S::Key3, Result<S::Output, S::Err>) -> Result<O, E>,
{
    type SnapShot = Convert<S::SnapShot, F>;
    type SnapShotErr = S::SnapShotErr;

    #[inline]
    fn take_snapshot<'a, It>(&self, keys: It) -> Result<Self::SnapShot, Self::SnapShotErr>
    where
        It: IntoIterator<Item = (&'a Self::Key1, &'a Self::Key2, &'a Self::Key3)>,
        Self::Key1: 'a,
        Self::Key2: 'a,
        Self::Key3: 'a,
    {
        self.src
            .take_snapshot(keys)
            .map(|snap| Convert::new(self.node.lock().unwrap().desc(), snap, self.f.clone()))
    }
}

// =============================================================================
#[cfg(test)]
mod tests {
    use std::sync::Mutex;

    use maplit::hashmap;
    use rstest::{fixture, rstest};

    use crate::datasrc::{OnMemorySrc, OnMemorySrc2Args, OnMemorySrc3Args};

    use super::*;

    #[fixture]
    fn src_1arg() -> OnMemorySrc<String, i32> {
        let src = OnMemorySrc::with_data(
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
    fn src_2args() -> OnMemorySrc2Args<String, String, i32> {
        let src = OnMemorySrc2Args::with_data(
            "src",
            hashmap! {
                "a".to_owned() => hashmap! {
                    "x".to_owned() => 1,
                    "y".to_owned() => 2,
                },
                "b".to_owned() => hashmap! {
                    "x".to_owned() => 3,
                    "y".to_owned() => 4,
                },
                "c".to_owned() => hashmap! {
                    "x".to_owned() => 5,
                    "y".to_owned() => 6,
                },
            },
        );
        src
    }

    #[fixture]
    fn src_3args() -> OnMemorySrc3Args<String, String, String, i32> {
        let src = OnMemorySrc3Args::with_data(
            "src",
            hashmap! {
                "a".to_owned() => hashmap! {
                    "x".to_owned() => hashmap! {
                        "i".to_owned() => 1,
                        "j".to_owned() => 2,
                    },
                    "y".to_owned() => hashmap! {
                        "i".to_owned() => 3,
                        "j".to_owned() => 4,
                    },
                },
                "b".to_owned() => hashmap! {
                    "x".to_owned() => hashmap! {
                        "i".to_owned() => 5,
                        "j".to_owned() => 6,
                    },
                    "y".to_owned() => hashmap! {
                        "i".to_owned() => 7,
                        "j".to_owned() => 8,
                    },
                },
                "c".to_owned() => hashmap! {
                    "x".to_owned() => hashmap! {
                        "i".to_owned() => 9,
                        "j".to_owned() => 10,
                    },
                    "y".to_owned() => hashmap! {
                        "i".to_owned() => 11,
                        "j".to_owned() => 12,
                    },
                },
            },
        );
        src
    }

    #[rstest]
    fn test_map_clone(src_1arg: OnMemorySrc<String, i32>) {
        let mut src = src_1arg.map("map", |x| x * 2);
        let src_clone = src.clone();

        let id = src.id();
        let id_clone = src_clone.id();
        let state = src.state();
        let state_clone = src_clone.state();

        assert_ne!(id, id_clone);

        // cloned instance is independent from the original instance.
        src.inner_mut().insert("a".to_owned(), 100);
        assert_eq!(id, src.id());
        assert_eq!(id_clone, src_clone.id());
        assert_ne!(state, src.state());
        assert_eq!(state_clone, src_clone.state());
        assert_eq!(src.req(&"a".to_owned()).unwrap(), 200);
        assert_eq!(src_clone.req(&"a".to_owned()).unwrap(), 2);
    }

    #[rstest]
    fn test_map_tree(src_1arg: OnMemorySrc<String, i32>) {
        let src = src_1arg.map("hogehoge map", |x| x * 2);
        let Tree::Branch {
            desc,
            id,
            state,
            children,
        } = src.tree()
        else {
            panic!("unexpected tree");
        };

        assert_eq!(desc, "hogehoge map");
        assert_eq!(id, src.id());
        assert_eq!(state, src.state());
        assert_eq!(children.len(), 1);
        assert_eq!(children.iter().next().unwrap(), &src.inner().tree());
    }

    #[rstest]
    fn test_map_1arg(src_1arg: OnMemorySrc<String, i32>) {
        let src_1arg = Arc::new(Mutex::new(src_1arg));
        let src = src_1arg.clone().map("map", |x| x * 2);

        // ok
        assert_eq!(src.req(&"a".to_owned()).unwrap(), 2);
        assert_eq!(src.req(&"b".to_owned()).unwrap(), 4);
        assert_eq!(src.req(&"c".to_owned()).unwrap(), 6);

        // err
        assert!(src.req(&"d".to_owned()).is_err());

        // state change
        let current_state = src.state();
        let current_val = src.req(&"a".to_owned()).unwrap();

        src_1arg.lock().unwrap().insert("a".to_owned(), 100);
        let new_state = src.state();
        let new_val = src.req(&"a".to_owned()).unwrap();

        assert_ne!(current_state, new_state);
        assert_eq!(current_val, 2);
        assert_eq!(new_val, 200);
    }

    #[rstest]
    fn test_map_2args(src_2args: OnMemorySrc2Args<String, String, i32>) {
        let src_2args = Arc::new(Mutex::new(src_2args));
        let src = src_2args.clone().map("map", |x| x * 2);

        // ok
        assert_eq!(src.req(&"a".to_owned(), &"x".to_owned()).unwrap(), 2);
        assert_eq!(src.req(&"a".to_owned(), &"y".to_owned()).unwrap(), 4);
        assert_eq!(src.req(&"b".to_owned(), &"x".to_owned()).unwrap(), 6);
        assert_eq!(src.req(&"b".to_owned(), &"y".to_owned()).unwrap(), 8);
        assert_eq!(src.req(&"c".to_owned(), &"x".to_owned()).unwrap(), 10);
        assert_eq!(src.req(&"c".to_owned(), &"y".to_owned()).unwrap(), 12);

        // err
        assert!(src.req(&"d".to_owned(), &"x".to_owned()).is_err());
        assert!(src.req(&"a".to_owned(), &"z".to_owned()).is_err());

        // state change
        let current_state = src.state();
        let current_val = src.req(&"a".to_owned(), &"x".to_owned()).unwrap();

        src_2args
            .lock()
            .unwrap()
            .insert("a".to_owned(), "x".to_owned(), 100);

        let new_state = src.state();
        let new_val = src.req(&"a".to_owned(), &"x".to_owned()).unwrap();

        assert_ne!(current_state, new_state);
        assert_eq!(current_val, 2);
        assert_eq!(new_val, 200);
    }

    #[rstest]
    fn test_map_3args(src_3args: OnMemorySrc3Args<String, String, String, i32>) {
        let src_3args = Arc::new(Mutex::new(src_3args));
        let src = src_3args.clone().map("map", |x| x * 2);

        // ok
        assert_eq!(
            src.req(&"a".to_owned(), &"x".to_owned(), &"i".to_owned())
                .unwrap(),
            2
        );
        assert_eq!(
            src.req(&"a".to_owned(), &"x".to_owned(), &"j".to_owned())
                .unwrap(),
            4
        );
        assert_eq!(
            src.req(&"a".to_owned(), &"y".to_owned(), &"i".to_owned())
                .unwrap(),
            6
        );
        assert_eq!(
            src.req(&"a".to_owned(), &"y".to_owned(), &"j".to_owned())
                .unwrap(),
            8
        );
        assert_eq!(
            src.req(&"b".to_owned(), &"x".to_owned(), &"i".to_owned())
                .unwrap(),
            10
        );
        assert_eq!(
            src.req(&"b".to_owned(), &"x".to_owned(), &"j".to_owned())
                .unwrap(),
            12
        );
        assert_eq!(
            src.req(&"b".to_owned(), &"y".to_owned(), &"i".to_owned())
                .unwrap(),
            14
        );
        assert_eq!(
            src.req(&"b".to_owned(), &"y".to_owned(), &"j".to_owned())
                .unwrap(),
            16
        );
        assert_eq!(
            src.req(&"c".to_owned(), &"x".to_owned(), &"i".to_owned())
                .unwrap(),
            18
        );
        assert_eq!(
            src.req(&"c".to_owned(), &"x".to_owned(), &"j".to_owned())
                .unwrap(),
            20
        );
        assert_eq!(
            src.req(&"c".to_owned(), &"y".to_owned(), &"i".to_owned())
                .unwrap(),
            22
        );
        assert_eq!(
            src.req(&"c".to_owned(), &"y".to_owned(), &"j".to_owned())
                .unwrap(),
            24
        );

        // err
        assert!(src
            .req(&"d".to_owned(), &"x".to_owned(), &"i".to_owned())
            .is_err());
        assert!(src
            .req(&"a".to_owned(), &"z".to_owned(), &"i".to_owned())
            .is_err());
        assert!(src
            .req(&"a".to_owned(), &"x".to_owned(), &"k".to_owned())
            .is_err());

        // state change
        let current_state = src.state();
        let current_val = src
            .req(&"a".to_owned(), &"x".to_owned(), &"i".to_owned())
            .unwrap();

        src_3args
            .lock()
            .unwrap()
            .insert("a".to_owned(), "x".to_owned(), "i".to_owned(), 100);

        let new_state = src.state();
        let new_val = src
            .req(&"a".to_owned(), &"x".to_owned(), &"i".to_owned())
            .unwrap();

        assert_ne!(current_state, new_state);
        assert_eq!(current_val, 2);
        assert_eq!(new_val, 200);
    }

    #[rstest]
    fn test_map_take_snapshot_1arg(src_1arg: OnMemorySrc<String, i32>) {
        let src = src_1arg.map("map", |x| x * 2);
        let snap = src
            .take_snapshot(&["a".to_owned(), "b".to_owned()])
            .unwrap();

        assert_eq!(snap.req(&"a".to_owned()).unwrap(), 2);
        assert_eq!(snap.req(&"b".to_owned()).unwrap(), 4);
        assert!(snap.req(&"c".to_owned()).is_err());
    }

    #[rstest]
    fn test_map_take_snapshot_2arg(src_2args: OnMemorySrc2Args<String, String, i32>) {
        let src = src_2args.map("map", |x| x * 2);

        let keys = [
            ("a".to_owned(), "x".to_owned()),
            ("b".to_owned(), "x".to_owned()),
        ];
        let snap = src
            .take_snapshot(keys.iter().map(|(k1, k2)| (k1, k2)))
            .unwrap();

        assert_eq!(snap.req(&"a".to_owned(), &"x".to_owned()).unwrap(), 2);
        assert_eq!(snap.req(&"b".to_owned(), &"x".to_owned()).unwrap(), 6);
        assert!(snap.req(&"c".to_owned(), &"x".to_owned()).is_err());
    }

    #[rstest]
    fn test_map_take_snapshot_3arg(src_3args: OnMemorySrc3Args<String, String, String, i32>) {
        let src = src_3args.map("map", |x| x * 2);

        let keys = [
            ("a".to_owned(), "x".to_owned(), "i".to_owned()),
            ("b".to_owned(), "y".to_owned(), "j".to_owned()),
        ];
        let snap = src
            .take_snapshot(keys.iter().map(|(k1, k2, k3)| (k1, k2, k3)))
            .unwrap();

        assert_eq!(
            snap.req(&"a".to_owned(), &"x".to_owned(), &"i".to_owned())
                .unwrap(),
            2
        );
        assert_eq!(
            snap.req(&"b".to_owned(), &"y".to_owned(), &"j".to_owned())
                .unwrap(),
            16
        );
        assert!(snap
            .req(&"c".to_owned(), &"x".to_owned(), &"i".to_owned())
            .is_err());
    }

    #[rstest]
    fn test_map_err_clone(src_1arg: OnMemorySrc<String, i32>) {
        let mut src = src_1arg.map_err("map", |_| "error".to_owned());
        let src_clone = src.clone();

        let id = src.id();
        let id_clone = src_clone.id();
        let state = src.state();
        let state_clone = src_clone.state();

        assert_ne!(id, id_clone);

        // cloned instance is independent from the original instance.
        src.inner_mut().insert("a".to_owned(), 100);
        assert_eq!(id, src.id());
        assert_eq!(id_clone, src_clone.id());
        assert_ne!(state, src.state());
        assert_eq!(state_clone, src_clone.state());
        assert_eq!(src.req(&"a".to_owned()).unwrap(), 100);
        assert_eq!(src_clone.req(&"a".to_owned()).unwrap(), 1);
    }

    #[rstest]
    fn test_map_err_tree(src_1arg: OnMemorySrc<String, i32>) {
        let src = src_1arg.map_err("hogehoge map", |_| "error".to_owned());
        let Tree::Branch {
            desc,
            id,
            state,
            children,
        } = src.tree()
        else {
            panic!("unexpected tree");
        };

        assert_eq!(desc, "hogehoge map");
        assert_eq!(id, src.id());
        assert_eq!(state, src.state());
        assert_eq!(children.len(), 1);
        assert_eq!(children.iter().next().unwrap(), &src.inner().tree());
    }

    #[rstest]
    fn test_map_err_1arg(src_1arg: OnMemorySrc<String, i32>) {
        let src_1arg = Arc::new(Mutex::new(src_1arg));
        let src = src_1arg.clone().map_err("map", |_| "error".to_owned());

        // ok
        assert_eq!(src.req(&"a".to_owned()).unwrap(), 1);
        assert_eq!(src.req(&"b".to_owned()).unwrap(), 2);
        assert_eq!(src.req(&"c".to_owned()).unwrap(), 3);

        // err
        assert!(src.req(&"d".to_owned()).is_err());
        assert!(src.req(&"d".to_owned()).unwrap_err() == "error");

        // state change
        let current_state = src.state();
        let current_val = src.req(&"a".to_owned()).unwrap();

        src_1arg.lock().unwrap().insert("a".to_owned(), 100);
        let new_state = src.state();
        let new_val = src.req(&"a".to_owned()).unwrap();

        assert_ne!(current_state, new_state);
        assert_eq!(current_val, 1);
        assert_eq!(new_val, 100);
    }

    #[rstest]
    fn test_map_err_2args(src_2args: OnMemorySrc2Args<String, String, i32>) {
        let src_2args = Arc::new(Mutex::new(src_2args));
        let src = src_2args.clone().map_err("map", |_| "error".to_owned());

        // ok
        assert_eq!(src.req(&"a".to_owned(), &"x".to_owned()).unwrap(), 1);
        assert_eq!(src.req(&"a".to_owned(), &"y".to_owned()).unwrap(), 2);
        assert_eq!(src.req(&"b".to_owned(), &"x".to_owned()).unwrap(), 3);
        assert_eq!(src.req(&"b".to_owned(), &"y".to_owned()).unwrap(), 4);
        assert_eq!(src.req(&"c".to_owned(), &"x".to_owned()).unwrap(), 5);
        assert_eq!(src.req(&"c".to_owned(), &"y".to_owned()).unwrap(), 6);

        // err
        assert!(src.req(&"d".to_owned(), &"x".to_owned()).is_err());
        assert!(src.req(&"d".to_owned(), &"x".to_owned()).unwrap_err() == "error");
        assert!(src.req(&"a".to_owned(), &"z".to_owned()).is_err());
        assert!(src.req(&"a".to_owned(), &"z".to_owned()).unwrap_err() == "error");

        // state change
        let current_state = src.state();
        let current_val = src.req(&"a".to_owned(), &"x".to_owned()).unwrap();

        src_2args
            .lock()
            .unwrap()
            .insert("a".to_owned(), "x".to_owned(), 100);
        let new_state = src.state();
        let new_val = src.req(&"a".to_owned(), &"x".to_owned()).unwrap();

        assert_ne!(current_state, new_state);
        assert_eq!(current_val, 1);
        assert_eq!(new_val, 100);
    }

    #[rstest]
    fn test_map_err_3args(src_3args: OnMemorySrc3Args<String, String, String, i32>) {
        let src_3args = Arc::new(Mutex::new(src_3args));
        let src = src_3args.clone().map_err("map", |_| "error".to_owned());

        // ok
        assert_eq!(
            src.req(&"a".to_owned(), &"x".to_owned(), &"i".to_owned())
                .unwrap(),
            1
        );
        assert_eq!(
            src.req(&"a".to_owned(), &"x".to_owned(), &"j".to_owned())
                .unwrap(),
            2
        );
        assert_eq!(
            src.req(&"a".to_owned(), &"y".to_owned(), &"i".to_owned())
                .unwrap(),
            3
        );
        assert_eq!(
            src.req(&"a".to_owned(), &"y".to_owned(), &"j".to_owned())
                .unwrap(),
            4
        );
        assert_eq!(
            src.req(&"b".to_owned(), &"x".to_owned(), &"i".to_owned())
                .unwrap(),
            5
        );
        assert_eq!(
            src.req(&"b".to_owned(), &"x".to_owned(), &"j".to_owned())
                .unwrap(),
            6
        );
        assert_eq!(
            src.req(&"b".to_owned(), &"y".to_owned(), &"i".to_owned())
                .unwrap(),
            7
        );
        assert_eq!(
            src.req(&"b".to_owned(), &"y".to_owned(), &"j".to_owned())
                .unwrap(),
            8
        );
        assert_eq!(
            src.req(&"c".to_owned(), &"x".to_owned(), &"i".to_owned())
                .unwrap(),
            9
        );
        assert_eq!(
            src.req(&"c".to_owned(), &"x".to_owned(), &"j".to_owned())
                .unwrap(),
            10
        );
        assert_eq!(
            src.req(&"c".to_owned(), &"y".to_owned(), &"i".to_owned())
                .unwrap(),
            11
        );
        assert_eq!(
            src.req(&"c".to_owned(), &"y".to_owned(), &"j".to_owned())
                .unwrap(),
            12
        );

        // err
        assert!(src
            .req(&"d".to_owned(), &"x".to_owned(), &"i".to_owned())
            .is_err());
        assert!(
            src.req(&"d".to_owned(), &"x".to_owned(), &"i".to_owned())
                .unwrap_err()
                == "error"
        );
        assert!(src
            .req(&"a".to_owned(), &"z".to_owned(), &"i".to_owned())
            .is_err());
        assert!(
            src.req(&"a".to_owned(), &"z".to_owned(), &"i".to_owned())
                .unwrap_err()
                == "error"
        );
        assert!(src
            .req(&"a".to_owned(), &"x".to_owned(), &"k".to_owned())
            .is_err());
        assert!(
            src.req(&"a".to_owned(), &"x".to_owned(), &"k".to_owned())
                .unwrap_err()
                == "error"
        );

        // state change
        let current_state = src.state();
        let current_val = src
            .req(&"a".to_owned(), &"x".to_owned(), &"i".to_owned())
            .unwrap();

        src_3args
            .lock()
            .unwrap()
            .insert("a".to_owned(), "x".to_owned(), "i".to_owned(), 100);

        let new_state = src.state();
        let new_val = src
            .req(&"a".to_owned(), &"x".to_owned(), &"i".to_owned())
            .unwrap();

        assert_ne!(current_state, new_state);
        assert_eq!(current_val, 1);
        assert_eq!(new_val, 100);
    }

    #[rstest]
    fn test_map_err_take_snapshot_1arg(src_1arg: OnMemorySrc<String, i32>) {
        let src = src_1arg.map_err("map", |_| "error".to_owned());
        let snap = src
            .take_snapshot(&["a".to_owned(), "b".to_owned()])
            .unwrap();

        assert_eq!(snap.req(&"a".to_owned()).unwrap(), 1);
        assert!(snap.req(&"c".to_owned()).is_err());
        assert_eq!(snap.req(&"c".to_owned()).unwrap_err(), "error");
    }

    #[rstest]
    fn test_map_err_take_snapshot_2arg(src_2args: OnMemorySrc2Args<String, String, i32>) {
        let src = src_2args.map_err("map", |_| "error".to_owned());

        let keys = [
            ("a".to_owned(), "x".to_owned()),
            ("b".to_owned(), "x".to_owned()),
        ];
        let snap = src
            .take_snapshot(keys.iter().map(|(k1, k2)| (k1, k2)))
            .unwrap();

        assert_eq!(snap.req(&"a".to_owned(), &"x".to_owned()).unwrap(), 1);
        assert!(snap.req(&"c".to_owned(), &"x".to_owned()).is_err());
        assert_eq!(
            snap.req(&"c".to_owned(), &"x".to_owned()).unwrap_err(),
            "error"
        );
    }

    #[rstest]
    fn test_map_err_take_snapshot_3arg(src_3args: OnMemorySrc3Args<String, String, String, i32>) {
        let src = src_3args.map_err("map", |_| "error".to_owned());

        let keys = [
            ("a".to_owned(), "x".to_owned(), "i".to_owned()),
            ("b".to_owned(), "y".to_owned(), "j".to_owned()),
        ];
        let snap = src
            .take_snapshot(keys.iter().map(|(k1, k2, k3)| (k1, k2, k3)))
            .unwrap();

        assert_eq!(
            snap.req(&"a".to_owned(), &"x".to_owned(), &"i".to_owned())
                .unwrap(),
            1
        );
        assert!(snap
            .req(&"c".to_owned(), &"x".to_owned(), &"i".to_owned())
            .is_err());
        assert!(
            snap.req(&"c".to_owned(), &"x".to_owned(), &"i".to_owned())
                .unwrap_err()
                == "error"
        );
    }

    #[rstest]
    fn test_convert_clone(src_1arg: OnMemorySrc<String, i32>) {
        let mut src = src_1arg.convert("convert", |_, r| r);
        let src_clone = src.clone();

        let id = src.id();
        let id_clone = src_clone.id();
        let state = src.state();
        let state_clone = src_clone.state();

        assert_ne!(id, id_clone);

        // cloned instance is independent from the original instance.
        src.inner_mut().insert("a".to_owned(), 100);
        assert_eq!(id, src.id());
        assert_eq!(id_clone, src_clone.id());
        assert_ne!(state, src.state());
        assert_eq!(state_clone, src_clone.state());
        assert_eq!(src.req(&"a".to_owned()).unwrap(), 100);
        assert_eq!(src_clone.req(&"a".to_owned()).unwrap(), 1);
    }

    #[rstest]
    fn test_convert_tree(src_1arg: OnMemorySrc<String, i32>) {
        let src = src_1arg.convert("hogehoge convert", |_, r| r);
        let Tree::Branch {
            desc,
            id,
            state,
            children,
        } = src.tree()
        else {
            panic!("unexpected tree");
        };

        assert_eq!(desc, "hogehoge convert");
        assert_eq!(id, src.id());
        assert_eq!(state, src.state());
        assert_eq!(children.len(), 1);
        assert_eq!(children.iter().next().unwrap(), &src.inner().tree());
    }

    #[rstest]
    fn test_convert_1arg(src_1arg: OnMemorySrc<String, i32>) {
        let src_1arg = Arc::new(Mutex::new(src_1arg));
        let src = src_1arg.clone().convert("convert", |s, r| match r {
            Ok(x) => {
                if x % 2 == 0 || s == "a" {
                    Ok(x)
                } else {
                    Err("error".to_owned())
                }
            }
            Err(_) => Err("downstream error".to_owned()),
        });

        // ok
        assert_eq!(src.req(&"a".to_owned()).unwrap(), 1);
        assert_eq!(src.req(&"b".to_owned()).unwrap(), 2);

        // err
        assert!(src.req(&"c".to_owned()).is_err());
        assert!(src.req(&"c".to_owned()).unwrap_err() == "error");

        assert!(src.req(&"d".to_owned()).is_err());
        assert!(src.req(&"d".to_owned()).unwrap_err() == "downstream error");

        // state change
        let current_state = src.state();
        let current_val = src.req(&"a".to_owned()).unwrap();

        src_1arg.lock().unwrap().insert("a".to_owned(), 100);
        let new_state = src.state();
        let new_val = src.req(&"a".to_owned()).unwrap();

        assert_ne!(current_state, new_state);
        assert_eq!(current_val, 1);
        assert_eq!(new_val, 100);
    }

    #[rstest]
    fn test_convert_2args(src_2args: OnMemorySrc2Args<String, String, i32>) {
        let src_2args = Arc::new(Mutex::new(src_2args));
        let src = src_2args.clone().convert("convert", |s1, _, r| match r {
            Ok(x) => {
                if x % 2 == 0 {
                    if s1 == "b" {
                        Err("b!".to_owned())
                    } else {
                        Ok(x)
                    }
                } else {
                    Err("error".to_owned())
                }
            }
            Err(_) => Err("downstream error".to_owned()),
        });

        // ok
        assert_eq!(src.req(&"a".to_owned(), &"y".to_owned()).unwrap(), 2);
        assert_eq!(src.req(&"c".to_owned(), &"y".to_owned()).unwrap(), 6);

        // err
        assert!(src.req(&"a".to_owned(), &"x".to_owned()).is_err());
        assert!(src.req(&"a".to_owned(), &"x".to_owned()).unwrap_err() == "error");

        assert!(src.req(&"b".to_owned(), &"y".to_owned()).is_err());
        assert!(src.req(&"b".to_owned(), &"y".to_owned()).unwrap_err() == "b!");

        assert!(src.req(&"d".to_owned(), &"x".to_owned()).is_err());
        assert!(src.req(&"d".to_owned(), &"x".to_owned()).unwrap_err() == "downstream error");

        // state change
        let current_state = src.state();
        let current_val = src.req(&"a".to_owned(), &"y".to_owned()).unwrap();

        src_2args
            .lock()
            .unwrap()
            .insert("a".to_owned(), "y".to_owned(), 100);
        let new_state = src.state();
        let new_val = src.req(&"a".to_owned(), &"y".to_owned()).unwrap();

        assert_ne!(current_state, new_state);
        assert_eq!(current_val, 2);
        assert_eq!(new_val, 100);
    }

    #[rstest]
    fn test_convert_3args(src_3args: OnMemorySrc3Args<String, String, String, i32>) {
        let src_3args = Arc::new(Mutex::new(src_3args));
        let src = src_3args
            .clone()
            .convert("convert", |s1, _, s3, r| match r {
                Ok(x) => {
                    if s3 == "j" {
                        let mult = match s1.as_str() {
                            "a" => 3,
                            "b" => 2,
                            _ => -1,
                        };
                        Ok(x * mult)
                    } else {
                        Err("error".to_owned())
                    }
                }
                Err(_) => Err("downstream error".to_owned()),
            });

        // ok
        assert_eq!(
            src.req(&"a".to_owned(), &"x".to_owned(), &"j".to_owned())
                .unwrap(),
            6
        );
        assert_eq!(
            src.req(&"b".to_owned(), &"x".to_owned(), &"j".to_owned())
                .unwrap(),
            12
        );
        assert_eq!(
            src.req(&"c".to_owned(), &"x".to_owned(), &"j".to_owned())
                .unwrap(),
            -10
        );

        // err
        assert!(src
            .req(&"a".to_owned(), &"x".to_owned(), &"i".to_owned())
            .is_err());
        assert!(
            src.req(&"a".to_owned(), &"x".to_owned(), &"i".to_owned())
                .unwrap_err()
                == "error"
        );

        assert!(src
            .req(&"a".to_owned(), &"y".to_owned(), &"i".to_owned())
            .is_err());
        assert!(
            src.req(&"a".to_owned(), &"y".to_owned(), &"i".to_owned())
                .unwrap_err()
                == "error"
        );

        assert!(src
            .req(&"d".to_owned(), &"x".to_owned(), &"i".to_owned())
            .is_err());
        assert!(
            src.req(&"d".to_owned(), &"x".to_owned(), &"i".to_owned())
                .unwrap_err()
                == "downstream error"
        );

        // state change
        let current_state = src.state();
        let current_val = src
            .req(&"a".to_owned(), &"x".to_owned(), &"j".to_owned())
            .unwrap();

        src_3args
            .lock()
            .unwrap()
            .insert("a".to_owned(), "x".to_owned(), "j".to_owned(), 100);

        let new_state = src.state();
        let new_val = src
            .req(&"a".to_owned(), &"x".to_owned(), &"j".to_owned())
            .unwrap();

        assert_ne!(current_state, new_state);
        assert_eq!(current_val, 6);
        assert_eq!(new_val, 300);
    }

    #[rstest]
    fn test_convert_take_snapshot_1arg(src_1arg: OnMemorySrc<String, i32>) {
        let src = src_1arg.convert("convert", |k, r| match r {
            Ok(_) => Err(k.to_owned()),
            Err(_) => Ok(42),
        });
        let snap = src
            .take_snapshot(&["a".to_owned(), "b".to_owned()])
            .unwrap();

        assert!(snap.req(&"a".to_owned()).is_err());
        assert!(snap.req(&"a".to_owned()).unwrap_err() == "a");
        assert!(snap.req(&"b".to_owned()).is_err());
        assert!(snap.req(&"b".to_owned()).unwrap_err() == "b");
        assert_eq!(snap.req(&"c".to_owned()).unwrap(), 42);
    }

    #[rstest]
    fn test_convert_take_snapshot_2arg(src_2args: OnMemorySrc2Args<String, String, i32>) {
        let src = src_2args.convert("convert", |k1, k2, r| match r {
            Ok(_) => Err(format!("{}{}", k1, k2)),
            Err(_) => Ok(42),
        });

        let keys = [
            ("a".to_owned(), "x".to_owned()),
            ("b".to_owned(), "x".to_owned()),
        ];
        let snap = src
            .take_snapshot(keys.iter().map(|(k1, k2)| (k1, k2)))
            .unwrap();

        assert!(snap.req(&"a".to_owned(), &"x".to_owned()).is_err());
        assert!(snap.req(&"a".to_owned(), &"x".to_owned()).unwrap_err() == "ax");
        assert!(snap.req(&"b".to_owned(), &"x".to_owned()).is_err());
        assert!(snap.req(&"b".to_owned(), &"x".to_owned()).unwrap_err() == "bx");
        assert_eq!(snap.req(&"c".to_owned(), &"x".to_owned()).unwrap(), 42);
    }

    #[rstest]
    fn test_convert_take_snapshot_3arg(src_3args: OnMemorySrc3Args<String, String, String, i32>) {
        let src = src_3args.convert("convert", |k1, k2, k3, r| match r {
            Ok(_) => Err(format!("{}{}{}", k1, k2, k3)),
            Err(_) => Ok(42),
        });

        let keys = [
            ("a".to_owned(), "x".to_owned(), "i".to_owned()),
            ("b".to_owned(), "y".to_owned(), "j".to_owned()),
        ];
        let snap = src
            .take_snapshot(keys.iter().map(|(k1, k2, k3)| (k1, k2, k3)))
            .unwrap();

        assert!(snap
            .req(&"a".to_owned(), &"x".to_owned(), &"i".to_owned())
            .is_err());
        assert!(
            snap.req(&"a".to_owned(), &"x".to_owned(), &"i".to_owned())
                .unwrap_err()
                == "axi"
        );
        assert!(snap
            .req(&"b".to_owned(), &"y".to_owned(), &"j".to_owned())
            .is_err());
        assert!(
            snap.req(&"b".to_owned(), &"y".to_owned(), &"j".to_owned())
                .unwrap_err()
                == "byj"
        );
        assert_eq!(
            snap.req(&"c".to_owned(), &"x".to_owned(), &"i".to_owned())
                .unwrap(),
            42
        );
    }
}
