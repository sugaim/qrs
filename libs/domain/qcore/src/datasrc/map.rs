use std::sync::Arc;

use qcore_derive::Node;

use super::{
    _private::_UnaryPassThroughNode, node::DataSrc2Args, snapshot::TakeSnapshot3Args, DataSrc,
    DataSrc3Args, Node, NodeStateId, TakeSnapshot, TakeSnapshot2Args,
};

// -----------------------------------------------------------------------------
// Map
//
#[derive(Debug, Node)]
#[node(transparent = "core")]
pub struct Map<S, F> {
    core: Arc<_UnaryPassThroughNode<S>>,
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
    #[inline]
    fn _new(desc: impl Into<String>, src: S, f: Arc<F>) -> Self {
        Self {
            core: _UnaryPassThroughNode::new(src, desc),
            f,
        }
    }

    #[inline]
    pub fn new(desc: impl Into<String>, src: S, f: F) -> Self {
        Self::_new(desc, src, Arc::new(f))
    }
}

//
// methods
//
impl<S, F> Map<S, F> {
    #[inline]
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
        Ok((self.core.state(), (self.f)(output)))
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
        Ok((self.core.state(), (self.f)(output)))
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
        Ok((self.core.state(), (self.f)(output)))
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

    #[inline]
    fn take_snapshot<'a, It>(&self, keys: It) -> Result<Self::SnapShot, Self::SnapShotErr>
    where
        It: IntoIterator<Item = &'a K>,
        K: 'a,
    {
        let snap = self.core.src.take_snapshot(keys)?;
        Ok(Map::_new(self.core.desc(), snap, self.f.clone()))
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

    #[inline]
    fn take_snapshot<'a, It>(&self, keys: It) -> Result<Self::SnapShot, Self::SnapShotErr>
    where
        It: IntoIterator<Item = (&'a K1, &'a K2)>,
        K1: 'a,
        K2: 'a,
    {
        let snap = self.core.src.take_snapshot(keys)?;
        Ok(Map::_new(self.core.desc(), snap, self.f.clone()))
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

    #[inline]
    fn take_snapshot<'a, It>(&self, keys: It) -> Result<Self::SnapShot, Self::SnapShotErr>
    where
        It: IntoIterator<Item = (&'a K1, &'a K2, &'a K3)>,
        K1: 'a,
        K2: 'a,
        K3: 'a,
    {
        let snap = self.core.src.take_snapshot(keys)?;
        Ok(Map::_new(self.core.desc(), snap, self.f.clone()))
    }
}

// -----------------------------------------------------------------------------
// MapErr
//
#[derive(Debug, Node)]
#[node(transparent = "core")]
pub struct MapErr<S, F> {
    core: Arc<_UnaryPassThroughNode<S>>,
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
    #[inline]
    fn _new(desc: impl Into<String>, src: S, f: Arc<F>) -> Self {
        Self {
            core: _UnaryPassThroughNode::new(src, desc),
            f,
        }
    }

    #[inline]
    pub fn new(desc: impl Into<String>, src: S, f: F) -> Self {
        Self::_new(desc, src, Arc::new(f))
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

    #[inline]
    fn take_snapshot<'a, It>(&self, keys: It) -> Result<Self::SnapShot, Self::SnapShotErr>
    where
        It: IntoIterator<Item = &'a K>,
        K: 'a,
    {
        let snap = self.core.src.take_snapshot(keys)?;
        Ok(MapErr::_new(self.core.desc(), snap, self.f.clone()))
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

    #[inline]
    fn take_snapshot<'a, It>(&self, keys: It) -> Result<Self::SnapShot, Self::SnapShotErr>
    where
        It: IntoIterator<Item = (&'a K1, &'a K2)>,
        K1: 'a,
        K2: 'a,
    {
        let snap = self.core.src.take_snapshot(keys)?;
        Ok(MapErr::_new(self.core.desc(), snap, self.f.clone()))
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

    #[inline]
    fn take_snapshot<'a, It>(&self, keys: It) -> Result<Self::SnapShot, Self::SnapShotErr>
    where
        It: IntoIterator<Item = (&'a K1, &'a K2, &'a K3)>,
        K1: 'a,
        K2: 'a,
        K3: 'a,
    {
        let snap = self.core.src.take_snapshot(keys)?;
        Ok(MapErr::_new(self.core.desc(), snap, self.f.clone()))
    }
}

// -----------------------------------------------------------------------------
// Convert
//
#[derive(Debug, Node)]
#[node(transparent = "core")]
pub struct Convert<S, F> {
    core: Arc<_UnaryPassThroughNode<S>>,
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
    fn _new(desc: impl Into<String>, src: S, f: Arc<F>) -> Self {
        Self {
            core: _UnaryPassThroughNode::new(src, desc),
            f,
        }
    }

    #[inline]
    pub fn new(desc: impl Into<String>, src: S, f: F) -> Self {
        Self::_new(desc, src, Arc::new(f))
    }
}

//
// methods
//
impl<S, F> Convert<S, F> {
    #[inline]
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
            Ok((_, output)) => (self.f)(Ok(output)).map(|output| (self.core.state(), output)),
            Err(err) => (self.f)(Err(err)).map(|o| (self.core.state(), o)),
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
            Ok((_, output)) => (self.f)(Ok(output)).map(|output| (self.core.state(), output)),
            Err(err) => (self.f)(Err(err)).map(|o| (self.core.state(), o)),
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
            Ok((_, output)) => (self.f)(Ok(output)).map(|output| (self.core.state(), output)),
            Err(err) => (self.f)(Err(err)).map(|o| (self.core.state(), o)),
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

    #[inline]
    fn take_snapshot<'a, It>(&self, keys: It) -> Result<Self::SnapShot, Self::SnapShotErr>
    where
        It: IntoIterator<Item = &'a K>,
        K: 'a,
    {
        let snap = self.core.src.take_snapshot(keys)?;
        Ok(Convert::_new(self.core.desc(), snap, self.f.clone()))
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

    #[inline]
    fn take_snapshot<'a, It>(&self, keys: It) -> Result<Self::SnapShot, Self::SnapShotErr>
    where
        It: IntoIterator<Item = (&'a K1, &'a K2)>,
        K1: 'a,
        K2: 'a,
    {
        let snap = self.core.src.take_snapshot(keys)?;
        Ok(Convert::_new(self.core.desc(), snap, self.f.clone()))
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

    #[inline]
    fn take_snapshot<'a, It>(&self, keys: It) -> Result<Self::SnapShot, Self::SnapShotErr>
    where
        It: IntoIterator<Item = (&'a K1, &'a K2, &'a K3)>,
        K1: 'a,
        K2: 'a,
        K3: 'a,
    {
        let snap = self.core.src.take_snapshot(keys)?;
        Ok(Convert::_new(self.core.desc(), snap, self.f.clone()))
    }
}

// =============================================================================
#[cfg(test)]
mod tests {
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
}
