use std::sync::Arc;

use qcore_derive::Node;

use super::{
    node::DataSrc2Args, private::_UnaryNode, snapshot::TakeSnapshot3Args, DataSrc, DataSrc3Args,
    Node, NodeInfo, NodeStateId, StateRecorder, TakeSnapshot, TakeSnapshot2Args,
};

// -----------------------------------------------------------------------------
// WithLogger
//
#[derive(Debug, Node)]
#[node(transparent = "core")]
pub struct WithLogger<S, F> {
    core: Arc<_UnaryNode<S>>,
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
        let core = Arc::new(_UnaryNode { src, states, info });
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
            core: Arc::new(_UnaryNode {
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
            core: Arc::new(_UnaryNode {
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
            core: Arc::new(_UnaryNode {
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
    use std::sync::{Arc, Mutex};

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
