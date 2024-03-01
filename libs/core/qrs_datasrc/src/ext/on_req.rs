use std::sync::{Mutex, Weak};

use qrs_datasrc_derive::DebugTree;

use crate::{DataSrc, DataSrc2Args, DataSrc3Args, Observer, PassThroughNode, Subject};

// -----------------------------------------------------------------------------
// WhenReq
//
#[derive(Debug, DebugTree)]
#[debug_tree(_use_from_qrs_datasrc, desc_field = "desc")]
pub struct OnReq<S, F> {
    desc: String,
    #[debug_tree(subtree)]
    src: S,
    f: F,
    node: PassThroughNode<(), ()>,
}

//
// construction
//
impl<S, F> OnReq<S, F>
where
    S: Subject,
{
    /// Create a new map.
    #[inline]
    pub(crate) fn new(mut src: S, f: F) -> Self {
        let (node, detector) = PassThroughNode::state_pass_through_unary(None);
        src.reg_observer(detector);
        let desc = "with action on request".to_string();
        OnReq { src, f, node, desc }
    }

    /// Add a description
    #[inline]
    pub fn with_desc(self, desc: impl Into<String>) -> Self {
        OnReq {
            desc: desc.into(),
            ..self
        }
    }
}

impl<S, F> Clone for OnReq<S, F>
where
    S: Subject + Clone,
    F: Clone,
{
    #[inline]
    fn clone(&self) -> Self {
        Self::new(self.src.clone(), self.f.clone()).with_desc(&self.desc)
    }
}

//
// methods
//
impl<S, F> OnReq<S, F> {
    /// Get the inner data source
    #[inline]
    pub fn inner(&self) -> &S {
        &self.src
    }

    /// Get the mutable reference to the inner data source
    #[inline]
    pub fn inner_mut(&mut self) -> &mut S {
        &mut self.src
    }

    /// Unwrap the inner data source
    #[inline]
    pub fn into_inner(self) -> S {
        self.src
    }
}

impl<S: Subject, F> Subject for OnReq<S, F> {
    #[inline]
    fn reg_observer(&mut self, observer: Weak<Mutex<dyn Observer>>) {
        self.node.reg_observer(observer);
    }

    #[inline]
    fn rm_observer(&mut self, observer: &Weak<Mutex<dyn Observer>>) {
        self.node.rm_observer(observer);
    }
}

impl<S, F> DataSrc for OnReq<S, F>
where
    S: DataSrc,
    F: Fn(&S::Key, &Result<S::Output, S::Err>),
{
    type Key = S::Key;
    type Output = S::Output;
    type Err = S::Err;

    #[inline]
    fn req(&self, key: &Self::Key) -> Result<Self::Output, Self::Err> {
        let res = self.src.req(key);
        (self.f)(key, &res);
        res
    }
}

impl<S, F> DataSrc2Args for OnReq<S, F>
where
    S: DataSrc2Args,
    F: Fn(&S::Key1, &S::Key2, &Result<S::Output, S::Err>),
{
    type Key1 = S::Key1;
    type Key2 = S::Key2;
    type Output = S::Output;
    type Err = S::Err;

    #[inline]
    fn req(&self, key1: &Self::Key1, key2: &Self::Key2) -> Result<Self::Output, Self::Err> {
        let res = self.src.req(key1, key2);
        (self.f)(key1, key2, &res);
        res
    }
}

impl<S, F> DataSrc3Args for OnReq<S, F>
where
    S: DataSrc3Args,
    F: Fn(&S::Key1, &S::Key2, &S::Key3, &Result<S::Output, S::Err>),
{
    type Key1 = S::Key1;
    type Key2 = S::Key2;
    type Key3 = S::Key3;
    type Output = S::Output;
    type Err = S::Err;

    #[inline]
    fn req(
        &self,
        key1: &Self::Key1,
        key2: &Self::Key2,
        key3: &Self::Key3,
    ) -> Result<Self::Output, Self::Err> {
        let res = self.src.req(key1, key2, key3);
        (self.f)(key1, key2, key3, &res);
        res
    }
}
