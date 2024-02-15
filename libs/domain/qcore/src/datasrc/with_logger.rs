use std::sync::{Arc, Mutex, Weak};

use maplit::btreeset;
use qcore_derive::Listener;

use super::{
    _private::_UnaryPassThroughNode, node::DataSrc2Args, snapshot::TakeSnapshot3Args, DataSrc,
    DataSrc3Args, Listener, NodeId, Notifier, StateId, TakeSnapshot, TakeSnapshot2Args, Tree,
};

// -----------------------------------------------------------------------------
// WithLogger
//
#[derive(Debug, Listener)]
#[listener(transparent = "node")]
pub struct WithLogger<S, F> {
    node: Arc<Mutex<_UnaryPassThroughNode>>,
    src: S,
    logger: F,
}

//
// construction
//

impl<S: Notifier, F> WithLogger<S, F> {
    #[inline]
    pub fn new(desc: impl Into<String>, mut src: S, logger: F) -> Self {
        let node = _UnaryPassThroughNode::new_and_reg(desc, &mut src);
        Self { node, src, logger }
    }
}

impl<S: Clone + Notifier, F: Clone> Clone for WithLogger<S, F> {
    #[inline]
    fn clone(&self) -> Self {
        Self::new(
            self.node.lock().unwrap().desc(),
            self.src.clone(),
            self.logger.clone(),
        )
    }
}

//
// methods
//
impl<S, F> WithLogger<S, F> {
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

impl<S: Notifier, F: 'static + Send + Sync> Notifier for WithLogger<S, F> {
    #[inline]
    fn id(&self) -> NodeId {
        self.node.lock().unwrap().id()
    }

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

impl<S, F> DataSrc for WithLogger<S, F>
where
    S: DataSrc,
    F: 'static + Send + Sync + Fn(&S::Key, &Result<S::Output, S::Err>),
{
    type Key = S::Key;
    type Output = S::Output;
    type Err = S::Err;

    #[inline]
    fn req(&self, key: &Self::Key) -> Result<Self::Output, Self::Err> {
        let result = self.src.req(key);
        (self.logger)(key, &result);
        result
    }
}

impl<S, F> DataSrc2Args for WithLogger<S, F>
where
    S: DataSrc2Args,
    F: 'static + Send + Sync + Fn(&S::Key1, &S::Key2, &Result<S::Output, S::Err>),
{
    type Key1 = S::Key1;
    type Key2 = S::Key2;
    type Output = S::Output;
    type Err = S::Err;

    #[inline]
    fn req(&self, key1: &S::Key1, key2: &S::Key2) -> Result<Self::Output, Self::Err> {
        let result = self.src.req(key1, key2);
        (self.logger)(key1, key2, &result);
        result
    }
}

impl<S, F> DataSrc3Args for WithLogger<S, F>
where
    S: DataSrc3Args,
    F: 'static + Send + Sync + Fn(&S::Key1, &S::Key2, &S::Key3, &Result<S::Output, S::Err>),
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
        let result = self.src.req(key1, key2, key3);
        (self.logger)(key1, key2, key3, &result);
        result
    }
}

impl<S, F> TakeSnapshot for WithLogger<S, F>
where
    S: TakeSnapshot,
    F: 'static + Send + Sync + Clone + Fn(&S::Key, &Result<S::Output, S::Err>),
{
    type SnapShot = WithLogger<S::SnapShot, F>;
    type SnapShotErr = S::SnapShotErr;

    fn take_snapshot<'a, It>(&self, keys: It) -> Result<Self::SnapShot, Self::SnapShotErr>
    where
        It: IntoIterator<Item = &'a S::Key>,
        S::Key: 'a,
    {
        self.src.take_snapshot(keys).map(|snap| {
            WithLogger::new(self.node.lock().unwrap().desc(), snap, self.logger.clone())
        })
    }
}

impl<S, F> TakeSnapshot2Args for WithLogger<S, F>
where
    S: TakeSnapshot2Args,
    F: 'static + Send + Sync + Clone + Fn(&S::Key1, &S::Key2, &Result<S::Output, S::Err>),
{
    type SnapShot = WithLogger<S::SnapShot, F>;
    type SnapShotErr = S::SnapShotErr;

    fn take_snapshot<'a, It>(&self, keys: It) -> Result<Self::SnapShot, Self::SnapShotErr>
    where
        It: IntoIterator<Item = (&'a S::Key1, &'a S::Key2)>,
        S::Key1: 'a,
        S::Key2: 'a,
    {
        self.src.take_snapshot(keys).map(|snap| {
            WithLogger::new(self.node.lock().unwrap().desc(), snap, self.logger.clone())
        })
    }
}

impl<S, F> TakeSnapshot3Args for WithLogger<S, F>
where
    S: TakeSnapshot3Args,
    F: 'static + Send + Sync + Clone + Fn(&S::Key1, &S::Key2, &S::Key3, &Result<S::Output, S::Err>),
{
    type SnapShot = WithLogger<S::SnapShot, F>;
    type SnapShotErr = S::SnapShotErr;

    fn take_snapshot<'a, It>(&self, keys: It) -> Result<Self::SnapShot, Self::SnapShotErr>
    where
        It: IntoIterator<Item = (&'a S::Key1, &'a S::Key2, &'a S::Key3)>,
        S::Key1: 'a,
        S::Key2: 'a,
        S::Key3: 'a,
    {
        self.src.take_snapshot(keys).map(|snap| {
            WithLogger::new(self.node.lock().unwrap().desc(), snap, self.logger.clone())
        })
    }
}

// =============================================================================
#[cfg(test)]
mod tests {
    use std::sync::{Arc, Mutex};

    use maplit::hashmap;
    use rstest::{fixture, rstest};

    use crate::datasrc::{OnMemorySrc, OnMemorySrc2Args, OnMemorySrc3Args};

    use super::*;

    #[fixture]
    fn src_1arg() -> OnMemorySrc<&'static str, i32> {
        let src = OnMemorySrc::with_data(
            "src",
            hashmap! {
                "a" => 1,
                "b" => 2,
                "c" => 3,
            },
        );
        src
    }

    #[fixture]
    fn src_2args() -> OnMemorySrc2Args<&'static str, &'static str, i32> {
        let src = OnMemorySrc2Args::with_data(
            "src",
            hashmap! {
                "a" => hashmap! {
                    "x" => 1,
                    "y" => 2,
                },
                "b" => hashmap! {
                    "x" => 3,
                    "y" => 4,
                },
                "c" => hashmap! {
                    "x" => 5,
                    "y" => 6,
                },
            },
        );
        src
    }

    #[fixture]
    fn src_3args() -> OnMemorySrc3Args<&'static str, &'static str, &'static str, i32> {
        let src = OnMemorySrc3Args::with_data(
            "src",
            hashmap! {
                "a" => hashmap! {
                    "x" => hashmap! {
                        "i" => 1,
                        "j" => 2,
                    },
                    "y" => hashmap! {
                        "i" => 3,
                        "j" => 4,
                    },
                },
                "b" => hashmap! {
                    "x" => hashmap! {
                        "i" => 5,
                        "j" => 6,
                    },
                    "y" => hashmap! {
                        "i" => 7,
                        "j" => 8,
                    },
                },
                "c" => hashmap! {
                    "x" => hashmap! {
                        "i" => 9,
                        "j" => 10,
                    },
                    "y" => hashmap! {
                        "i" => 11,
                        "j" => 12,
                    },
                },
            },
        );
        src
    }

    #[rstest]
    fn test_with_logger_1arg(src_1arg: OnMemorySrc<&'static str, i32>) {
        let src_1arg = Arc::new(Mutex::new(src_1arg));
        let (reader, src) = {
            let msg = Arc::new(Mutex::new(String::new()));
            let reader = msg.clone();
            let src = src_1arg.clone().with_logger("with_logger", move |k, r| {
                let mut msg = msg.lock().unwrap();
                match r {
                    Ok(v) => *msg = format!("[ok] {}: {}", k, v),
                    Err(_) => *msg = format!("[ng] {}", k),
                }
            });
            (reader, src)
        };

        // ok
        assert_eq!(src.req(&"a").unwrap(), 1);
        assert_eq!(reader.lock().unwrap().as_str(), "[ok] a: 1");
        assert_eq!(src.req(&"b").unwrap(), 2);
        assert_eq!(reader.lock().unwrap().as_str(), "[ok] b: 2");
        assert_eq!(src.req(&"c").unwrap(), 3);
        assert_eq!(reader.lock().unwrap().as_str(), "[ok] c: 3");

        // err
        assert!(src.req(&"d").is_err());
        assert_eq!(reader.lock().unwrap().as_str(), "[ng] d");

        // state change
        let current_state = src.state();
        let current_val = src.req(&"a").unwrap();
        src_1arg.lock().unwrap().insert("a", 100);

        let new_state = src.state();
        let new_val = src.req(&"a").unwrap();

        assert_ne!(current_state, new_state);
        assert_eq!(current_val, 1);
        assert_eq!(new_val, 100);
        assert_eq!(reader.lock().unwrap().as_str(), "[ok] a: 100");
    }

    #[rstest]
    fn test_with_logger_2args(src_2args: OnMemorySrc2Args<&'static str, &'static str, i32>) {
        let src_2args = Arc::new(Mutex::new(src_2args));
        let (reader, src) = {
            let msg = Arc::new(Mutex::new(String::new()));
            let reader = msg.clone();
            let src = src_2args.clone().with_logger("logger", move |k1, k2, r| {
                let mut msg = msg.lock().unwrap();
                match r {
                    Ok(v) => *msg = format!("[ok] ({}, {}): {}", k1, k2, v),
                    Err(_) => *msg = format!("[ng] ({}, {})", k1, k2),
                }
            });
            (reader, src)
        };

        // ok
        assert_eq!(src.req(&"a", &"x").unwrap(), 1);
        assert_eq!(reader.lock().unwrap().as_str(), "[ok] (a, x): 1");
        assert_eq!(src.req(&"b", &"x").unwrap(), 3);
        assert_eq!(reader.lock().unwrap().as_str(), "[ok] (b, x): 3");
        assert_eq!(src.req(&"c", &"x").unwrap(), 5);
        assert_eq!(reader.lock().unwrap().as_str(), "[ok] (c, x): 5");

        // err
        assert!(src.req(&"d", &"x").is_err());
        assert_eq!(reader.lock().unwrap().as_str(), "[ng] (d, x)");

        // state change
        let current_state = src.state();
        let current_val = src.req(&"a", &"x").unwrap();

        src_2args.lock().unwrap().insert("a", "x", 100);
        let new_state = src.state();
        let new_val = src.req(&"a", &"x").unwrap();

        assert_ne!(current_state, new_state);
        assert_eq!(current_val, 1);
        assert_eq!(new_val, 100);
        assert_eq!(reader.lock().unwrap().as_str(), "[ok] (a, x): 100");
    }

    #[rstest]
    fn test_with_logger_3args(
        src_3args: OnMemorySrc3Args<&'static str, &'static str, &'static str, i32>,
    ) {
        let src_3args = Arc::new(Mutex::new(src_3args));
        let (reader, src) = {
            let msg = Arc::new(Mutex::new(String::new()));
            let reader = msg.clone();
            let src = src_3args.clone().with_logger("log", move |k1, k2, k3, r| {
                let mut msg = msg.lock().unwrap();
                match r {
                    Ok(v) => *msg = format!("[ok] ({}, {}, {}): {}", k1, k2, k3, v),
                    Err(_) => *msg = format!("[ng] ({}, {}, {})", k1, k2, k3),
                }
            });
            (reader, src)
        };

        // ok
        assert_eq!(src.req(&"a", &"x", &"i").unwrap(), 1);
        assert_eq!(reader.lock().unwrap().as_str(), "[ok] (a, x, i): 1");
        assert_eq!(src.req(&"b", &"x", &"i").unwrap(), 5);
        assert_eq!(reader.lock().unwrap().as_str(), "[ok] (b, x, i): 5");
        assert_eq!(src.req(&"c", &"x", &"i").unwrap(), 9);
        assert_eq!(reader.lock().unwrap().as_str(), "[ok] (c, x, i): 9");

        // err
        assert!(src.req(&"d", &"x", &"i").is_err());
        assert_eq!(reader.lock().unwrap().as_str(), "[ng] (d, x, i)");

        // state change
        let current_state = src.state();
        let current_val = src.req(&"a", &"x", &"i").unwrap();

        src_3args.lock().unwrap().insert("a", "x", "i", 100);
        let new_state = src.state();
        let new_val = src.req(&"a", &"x", &"i").unwrap();

        assert_ne!(current_state, new_state);
        assert_eq!(current_val, 1);
        assert_eq!(new_val, 100);
        assert_eq!(reader.lock().unwrap().as_str(), "[ok] (a, x, i): 100");
    }
}
