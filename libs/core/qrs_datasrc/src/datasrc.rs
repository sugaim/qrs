use std::{
    hash::Hash,
    num::NonZeroUsize,
    sync::{Arc, Mutex},
};

use crate::{CacheProxy, DebugTree, Map, OnGet};

// -----------------------------------------------------------------------------
// DataSrc
//

/// Interface for data source
pub trait DataSrc<Rq: ?Sized>: DebugTree {
    type Output;

    /// Get the data from the data source.
    fn get(&self, req: &Rq) -> anyhow::Result<Self::Output>;

    /// Add an action on getting the data.
    fn on_get<A>(self, action: A) -> OnGet<Self, A>
    where
        Self: Sized,
        A: Fn(&Rq, Result<&Self::Output, &anyhow::Error>),
    {
        OnGet::new(self, action)
    }

    /// Map the output.
    ///
    /// Even when the original data source is [`CacheableSrc`],
    /// the mapped data source is not for some cases.
    /// [`Map`] only implements [`CacheableSrc`] when the original data source is [`CacheableSrc`]
    /// and the mapping function is `fn` to avoid
    /// inconsistent etag cauased by the state of the mapping function.
    ///
    /// If you can ensure that the function is stateless,
    /// you can use [`Map::ensure_stateless_func`] to
    /// implements [`CacheableSrc`] for the mapped data source.
    fn map<O, F>(self, map: F) -> Map<Self, F>
    where
        Self: Sized,
        F: Fn(Self::Output) -> anyhow::Result<O>,
    {
        Map::new(self, map)
    }
}

impl<S, Rq> DataSrc<Rq> for Arc<S>
where
    S: ?Sized + DataSrc<Rq>,
    Rq: ?Sized,
{
    type Output = S::Output;

    #[inline]
    fn get(&self, req: &Rq) -> anyhow::Result<Self::Output> {
        self.as_ref().get(req)
    }
}

impl<S, Rq> DataSrc<Rq> for Mutex<S>
where
    S: ?Sized + DataSrc<Rq>,
    Rq: ?Sized,
{
    type Output = S::Output;

    #[inline]
    fn get(&self, req: &Rq) -> anyhow::Result<Self::Output> {
        self.lock().unwrap().get(req)
    }
}

// -----------------------------------------------------------------------------
// Response
//
/// Response of data source.
#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
#[cfg_attr(
    feature = "serde",
    derive(serde::Serialize, serde::Deserialize, schemars::JsonSchema),
    serde(rename_all = "snake_case", tag = "type"),
    schemars(description = "Response of data source")
)]
pub struct Response<T> {
    pub data: T,
    pub etag: String,
}

// -----------------------------------------------------------------------------
// CacheableSrc
//

/// Interface for cacheable data source(auto implemented for types that implement [`Etag`])
///
/// This trait combines [`Etag`] and [`DataSrc`] to provide a cacheable data source.
/// First, it provides an extended version of [`DataSrc::get`] that returns the data with its etag.
/// Second, it provides a method to validate cached data with the etag.
///
pub trait CacheableSrc<Rq: ?Sized>: DataSrc<Rq> {
    /// Publish an etag of the requested data.
    fn etag(&self, req: &Rq) -> anyhow::Result<String>;

    /// Get the data with its etag.
    #[inline]
    fn get_with_etag(&self, req: &Rq) -> anyhow::Result<Response<Self::Output>> {
        let etag = self.etag(req)?;
        let data = self.get(req)?;
        Ok(Response { data, etag })
    }

    /// Validate the etag and get the data if it is changed.
    #[inline]
    fn get_if_none_match(
        &self,
        req: &Rq,
        etag: &str,
    ) -> anyhow::Result<Option<Response<Self::Output>>> {
        let current = self.etag(req)?;
        if current == etag {
            Ok(None)
        } else {
            Ok(Some(Response {
                data: self.get(req)?,
                etag: current,
            }))
        }
    }

    /// Create a cache proxy for the data source.
    fn into_cache_proxy(
        self,
        cache_cap: Option<NonZeroUsize>,
    ) -> CacheProxy<Self, Rq::Owned, Self::Output>
    where
        Self: Sized,
        Rq: Eq + Hash + ToOwned,
        Rq::Owned: Eq + Hash,
        Self::Output: Clone,
    {
        CacheProxy::new(self, cache_cap)
    }
}

impl<S, Rq> CacheableSrc<Rq> for Arc<S>
where
    S: ?Sized + CacheableSrc<Rq>,
    Rq: ?Sized,
{
    #[inline]
    fn etag(&self, req: &Rq) -> anyhow::Result<String> {
        self.as_ref().etag(req)
    }

    #[inline]
    fn get_with_etag(&self, req: &Rq) -> anyhow::Result<Response<S::Output>> {
        self.as_ref().get_with_etag(req)
    }

    #[inline]
    fn get_if_none_match(
        &self,
        req: &Rq,
        etag: &str,
    ) -> anyhow::Result<Option<Response<S::Output>>> {
        self.as_ref().get_if_none_match(req, etag)
    }
}

impl<S, Rq> CacheableSrc<Rq> for Mutex<S>
where
    S: ?Sized + CacheableSrc<Rq>,
    Rq: ?Sized,
{
    #[inline]
    fn etag(&self, req: &Rq) -> anyhow::Result<String> {
        self.lock().unwrap().etag(req)
    }

    #[inline]
    fn get_with_etag(&self, req: &Rq) -> anyhow::Result<Response<S::Output>> {
        self.lock().unwrap().get_with_etag(req)
    }

    #[inline]
    fn get_if_none_match(
        &self,
        req: &Rq,
        etag: &str,
    ) -> anyhow::Result<Option<Response<S::Output>>> {
        self.lock().unwrap().get_if_none_match(req, etag)
    }
}

// =============================================================================
#[cfg(test)]
mod tests {
    use qrs_datasrc_derive::DebugTree;
    use rstest::rstest;

    use crate::*;
    use _test_util::*;

    #[derive(DebugTree)]
    #[debug_tree(_use_from_qrs_datasrc)]
    struct Wrapper(#[debug_tree(subtree)] MockSrc);

    impl DataSrc<str> for Wrapper {
        type Output = String;
        fn get(&self, req: &str) -> anyhow::Result<Self::Output> {
            self.0.get(req)
        }
    }
    impl CacheableSrc<str> for Wrapper {
        fn etag(&self, req: &str) -> anyhow::Result<String> {
            self.0.etag(req)
        }
    }

    #[test]
    fn test_default_impl_of_get_with_etag() {
        let mock = MockSrc::with_call_count(&CallCount {
            get: Some(1),
            etag: Some(1),
            ..CallCount::zero()
        });
        let mut src = Wrapper(mock);

        let res = src.get_with_etag("foo").unwrap();

        assert_eq!(res, MockSrc::to_res("foo"));
        src.0.checkpoint();
    }

    #[rstest]
    #[case(true)]
    #[case(false)]
    fn test_default_impl_of_get_if_not_match(#[case] etag_match: bool) {
        let mock = MockSrc::with_call_count(&CallCount {
            etag: Some(1),
            get: if etag_match { Some(0) } else { Some(1) },
            ..CallCount::zero()
        });
        let mut src = Wrapper(mock);
        let req = "foo";
        let etag = MockSrc::to_etag(if etag_match { "foo" } else { "bar" });

        let res = src.get_if_none_match(req, &etag).unwrap();

        if etag_match {
            assert!(res.is_none());
        } else {
            assert_eq!(res, Some(MockSrc::to_res("foo")));
        }
        src.0.checkpoint();
    }
}
