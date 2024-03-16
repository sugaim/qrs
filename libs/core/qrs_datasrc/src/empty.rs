use qrs_datasrc_derive::DebugTree;

use crate::{CacheableSrc, DataSrc, TakeSnapshot};

// -----------------------------------------------------------------------------
// EmptyDataSrc
//
#[derive(Debug, Clone, PartialEq, Eq, Hash, DebugTree)]
#[debug_tree(_use_from_qrs_datasrc, desc = "Empty data source")]
pub struct EmptyDataSrc<V> {
    msg: String,
    mkr: std::marker::PhantomData<V>,
}

//
// construction
//
impl<V> Default for EmptyDataSrc<V> {
    #[inline]
    fn default() -> Self {
        Self::new()
    }
}

impl<V> EmptyDataSrc<V> {
    #[inline]
    pub fn new() -> Self {
        Self {
            msg: "empty data source can not return anything".to_string(),
            mkr: std::marker::PhantomData,
        }
    }
}

//
// methods
//
impl<V> EmptyDataSrc<V> {
    /// Returns a reference to the message.
    #[inline]
    pub fn message(&self) -> &str {
        &self.msg
    }

    /// Sets the message to the given one.
    #[inline]
    pub fn set_msg(&mut self, msg: impl Into<String>) {
        self.msg = msg.into();
    }
}

impl<V, K: ?Sized> DataSrc<K> for EmptyDataSrc<V> {
    type Output = V;

    #[inline]
    fn get(&self, _: &K) -> anyhow::Result<Self::Output> {
        Err(anyhow::anyhow!(self.msg.clone()))
    }
}

impl<V, K: ?Sized> CacheableSrc<K> for EmptyDataSrc<V> {
    #[inline]
    fn etag(&self, _: &K) -> anyhow::Result<String> {
        Err(anyhow::anyhow!(self.msg.clone()))
    }
}

impl<V, K: ?Sized> TakeSnapshot<K> for EmptyDataSrc<V> {
    type Snapshot = Self;

    fn take_snapshot<'a, Rqs>(&self, _: Rqs) -> anyhow::Result<Self::Snapshot>
    where
        K: 'a,
        Rqs: IntoIterator<Item = &'a K>,
    {
        Err(anyhow::anyhow!(self.msg.clone()))
    }
}
