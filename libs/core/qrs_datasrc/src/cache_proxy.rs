use std::{
    borrow::Borrow,
    hash::{BuildHasher, Hash},
    num::NonZeroUsize,
    sync::Mutex,
};

use derivative::Derivative;
use lru::{DefaultHasher, LruCache};
use qrs_datasrc_derive::DebugTree;

use crate::{CacheableSrc, DataSrc, Response};

// -----------------------------------------------------------------------------
// CacheProxy
//
/// Naive cache proxy for data source.
///
/// This proxy uses [`CacheableSrc`] to cache the data.
/// That is, this proxy holds a data and its etag in the cache
/// and validate them with [`CacheableSrc::get_if_none_match`] method of the underlying data source
/// when a request is sent.
/// As a cache, this proxy uses [`lru::LruCache`].
///
/// Type parameters `K` and `V` are the stored key and value types, respectively.
/// Since we use hash map as the cache, `K` must implement [`Eq`] and [`Hash`].
/// Also, to return the cached data, `V` must implement [`Clone`].
///
/// This also supports some weak types, such as [`str`] as request type
/// when the underlying data source supports them.
/// But such a weak type must be related with `K` to check the key equality
/// and store the key in the cache.
/// So, a weak request type `Rq` must be convertible to `K` by [`ToOwned`] and
/// `K` must be convertible to `Rq` by [`Borrow`].
/// As an example of `Rq` and `K`, we can use [`str`] and [`String`], respectively.
#[derive(Derivative, DebugTree)]
#[debug_tree(_use_from_qrs_datasrc, desc = "cache proxy")]
#[derivative(Debug, PartialEq(bound = "Src: PartialEq"), Eq(bound = "Src: Eq"))]
pub struct CacheProxy<Src, K, V, S = DefaultHasher> {
    #[debug_tree(subtree)]
    src: Src,
    #[derivative(PartialEq = "ignore", Debug = "ignore")]
    cache: Mutex<LruCache<K, Response<V>, S>>,
}

//
// construction
//
impl<Src, K, V> CacheProxy<Src, K, V> {
    /// Create a new instance.
    /// With `cache_cap` set to `None`, the cache will have no limit.
    #[inline]
    pub fn new(src: Src, cache_cap: Option<NonZeroUsize>) -> Self
    where
        K: Eq + Hash,
    {
        Self {
            src,
            cache: match cache_cap {
                Some(cap) => LruCache::new(cap).into(),
                None => LruCache::unbounded().into(),
            },
        }
    }

    /// Create a new instance with specified request type.
    ///
    /// Since the constructor [`Self::new`] can not infer `K` and `V` from `Src`,
    /// users must specify `Src`, `K` and `V` explicitly, like `CacheProxy::<_, K, V>::new(...)`.
    ///
    /// This method accept the request type `Rq` of the underlying data source
    /// and infer `K` and `V` from it.
    #[inline]
    pub fn with_req_type<Rq>(src: Src, cache_cap: Option<NonZeroUsize>) -> Self
    where
        Rq: ?Sized + ToOwned<Owned = K>,
        Rq::Owned: Eq + Hash,
        Src: CacheableSrc<Rq, Output = V>,
    {
        Self::new(src, cache_cap)
    }
}

impl<Src, K, V, S> CacheProxy<Src, K, V, S>
where
    K: Eq + Hash,
    S: BuildHasher,
{
    /// Create a new instance with a custom hasher.
    /// With `cache_cap` set to `None`, the cache will have no limit.
    #[inline]
    pub fn with_hasher(src: Src, cache_cap: Option<NonZeroUsize>, hasher: S) -> Self {
        Self {
            src,
            cache: match cache_cap {
                Some(cap) => LruCache::with_hasher(cap, hasher).into(),
                None => LruCache::unbounded_with_hasher(hasher).into(),
            },
        }
    }

    /// Create a new instance with a custom hasher and specified request type.
    ///
    /// Since the constructor [`Self::with_hasher`] can not infer `K` and `V` from `Src`,
    /// users must specify `Src`, `K` and `V` explicitly, like `CacheProxy::<_, K, V>::with_hasher(...)`.
    ///
    /// This method accept the request type `Rq` of the underlying data source
    /// and infer `K` and `V` from it.
    #[inline]
    pub fn with_req_type_and_hasher<Rq>(
        src: Src,
        cache_cap: Option<NonZeroUsize>,
        hasher: S,
    ) -> Self
    where
        Rq: ?Sized + ToOwned<Owned = K>,
        Rq::Owned: Eq + Hash,
        Src: CacheableSrc<Rq, Output = V>,
    {
        Self::with_hasher(src, cache_cap, hasher)
    }
}

impl<Src, K, V, S> Clone for CacheProxy<Src, K, V, S>
where
    K: Eq + Hash,
    Src: Clone,
    S: Default + BuildHasher,
{
    #[inline]
    fn clone(&self) -> Self {
        Self::with_hasher(self.src.clone(), self.cache_capacity(), S::default())
    }
}

//
// methods
//
impl<Src, K, V, S> CacheProxy<Src, K, V, S>
where
    K: Eq + Hash,
    S: BuildHasher,
{
    /// Get the inner data source.
    #[inline]
    pub fn inner(&self) -> &Src {
        &self.src
    }

    /// Get the mutable reference to the inner data source.
    #[inline]
    pub fn inner_mut(&mut self) -> &mut Src {
        &mut self.src
    }

    /// Unwrap the inner data source.
    #[inline]
    pub fn into_inner(self) -> Src {
        self.src
    }

    /// Get the cache capacity.
    #[inline]
    pub fn cache_capacity(&self) -> Option<NonZeroUsize> {
        let sz = self.cache.lock().unwrap().cap();
        if sz == NonZeroUsize::MAX {
            None
        } else {
            Some(sz)
        }
    }

    /// Set the cache capacity.
    /// With `new_cap` set to `None`, the cache will have no limit.
    #[inline]
    pub fn set_cache_capacity(&mut self, new_cap: Option<NonZeroUsize>) {
        let mut cache = self.cache.lock().unwrap();
        cache.resize(new_cap.unwrap_or(NonZeroUsize::MAX));
    }

    /// Clear the cache.
    #[inline]
    pub fn clear_cache(&mut self) {
        self.cache.lock().unwrap().clear();
    }
}

impl<Src, S, V, Rq> DataSrc<Rq> for CacheProxy<Src, Rq::Owned, V, S>
where
    Rq: ?Sized + Eq + Hash + ToOwned,
    Src: CacheableSrc<Rq>,
    Rq::Owned: Eq + Hash + Borrow<Rq>,
    S: BuildHasher,
    Src::Output: Into<V> + Clone,
    V: Into<Src::Output> + Clone,
{
    type Output = Src::Output;

    #[inline]
    fn get(&self, req: &Rq) -> anyhow::Result<Self::Output> {
        self.get_with_etag(req).map(|res| res.data)
    }
}

impl<Src, S, V, Rq> CacheableSrc<Rq> for CacheProxy<Src, Rq::Owned, V, S>
where
    Rq: ?Sized + Eq + Hash + ToOwned,
    Src: CacheableSrc<Rq>,
    Rq::Owned: Eq + Hash + Borrow<Rq>,
    S: BuildHasher,
    Src::Output: Into<V> + Clone,
    V: Into<Src::Output> + Clone,
{
    #[inline]
    fn etag(&self, req: &Rq) -> anyhow::Result<String> {
        self.src.etag(req)
    }

    fn get_with_etag(&self, req: &Rq) -> anyhow::Result<Response<Self::Output>> {
        let mut cache = self.cache.lock().unwrap();
        let Some(cached) = cache.get(req) else {
            let res = self.src.get_with_etag(req)?;
            cache.put(
                req.to_owned(),
                Response {
                    data: res.data.clone().into(),
                    etag: res.etag.clone(),
                },
            );
            return Ok(res);
        };
        let fetched = match self.src.get_if_none_match(req, &cached.etag)? {
            None => {
                return Ok(Response {
                    data: cached.data.clone().into(),
                    etag: cached.etag.clone(),
                })
            }
            Some(fetched) => fetched,
        };
        cache.put(
            req.to_owned(),
            Response {
                data: fetched.data.clone().into(),
                etag: fetched.etag.clone(),
            },
        );
        Ok(fetched)
    }

    fn get_if_none_match(
        &self,
        req: &Rq,
        etag: &str,
    ) -> anyhow::Result<Option<Response<Self::Output>>> {
        let res = self.src.get_if_none_match(req, etag)?;
        let Some(res) = res else {
            return Ok(None);
        };
        let mut cache = self.cache.lock().unwrap();
        cache.put(
            req.to_owned(),
            Response {
                data: res.data.clone().into(),
                etag: res.etag.clone(),
            },
        );
        Ok(Some(res))
    }
}

// =============================================================================
#[cfg(test)]
mod tests {
    use std::num::NonZeroUsize;

    use rstest::rstest;

    use crate::*;
    use _test_util::*;

    #[test]
    fn test_desc() {
        let mock = MockSrc::with_call_count(&CallCount::zero());
        let mut src = CacheProxy::with_req_type::<str>(mock, None);

        let desc = src.desc();

        assert_eq!(desc, "cache proxy");
        src.src.checkpoint();
    }

    #[test]
    fn test_debug_tree() {
        let mock = MockSrc::with_call_count(&CallCount {
            debug_tree: Some(1),
            ..CallCount::zero()
        });
        let mut src = CacheProxy::with_req_type::<str>(mock, None);

        let tree = src.debug_tree();

        assert_eq!(
            tree,
            TreeInfo::Wrap {
                desc: "cache proxy".to_owned(),
                tp: std::any::type_name::<CacheProxy<MockSrc, String, String>>().to_owned(),
                child: TreeInfo::Leaf {
                    desc: MOCK_DESC.to_owned(),
                    tp: std::any::type_name::<MockSrc>().to_owned(),
                }
                .into(),
            }
        );
        src.src.checkpoint();
    }

    #[rstest]
    #[case(None)]
    #[case(Some(NonZeroUsize::new(5).unwrap()))]
    fn test_new(#[case] cache_cap: Option<NonZeroUsize>) {
        let mock = MockSrc::with_call_count(&CallCount::zero());

        let mut src = CacheProxy::with_req_type::<str>(mock, cache_cap);

        assert_eq!(src.cache_capacity(), cache_cap);
        src.src.checkpoint();
    }

    #[rstest]
    #[case(None, None)]
    #[case(None, Some(NonZeroUsize::new(5).unwrap()))]
    #[case(Some(NonZeroUsize::new(3).unwrap()), Some(NonZeroUsize::new(5).unwrap()))]
    #[case(Some(NonZeroUsize::new(3).unwrap()), None)]
    fn test_set_cache_capacity(
        #[case] from: Option<NonZeroUsize>,
        #[case] to: Option<NonZeroUsize>,
    ) {
        let mock = MockSrc::with_call_count(&CallCount::zero());
        let mut src = CacheProxy::with_req_type::<str>(mock, from);

        src.set_cache_capacity(to);

        assert_eq!(src.cache_capacity(), to);
        src.src.checkpoint();
    }

    #[test]
    fn test_clear_cache() {
        let mock = MockSrc::with_call_count(&CallCount {
            get_with_etag: Some(1),
            ..CallCount::zero()
        });
        let mut src = CacheProxy::with_req_type::<str>(mock, None);
        let _ = src.get("abc").unwrap(); // cache created. see `test_get_repeat`
        src.src.checkpoint();
        src.src.setup(&CallCount::zero());

        src.clear_cache();

        src.src.checkpoint(); // clear cache does not affect the underlying source.
        src.src.setup(&CallCount {
            get_with_etag: Some(1),
            ..CallCount::zero()
        });
        let _ = src.get("abc").unwrap(); // cache is already cleared, so get method is called again.
        src.src.checkpoint();
    }

    #[test]
    fn test_get() {
        // get
        let mock = MockSrc::with_call_count(&CallCount {
            get_with_etag: Some(1),
            ..CallCount::zero()
        });
        let mut src = CacheProxy::with_req_type::<str>(mock, None);

        let res = src.get("abc").unwrap();

        assert_eq!(res, MockSrc::to_res("abc").data);
        src.src.checkpoint();

        // get_with_etag
        let mock = MockSrc::with_call_count(&CallCount {
            get_with_etag: Some(1),
            ..CallCount::zero()
        });
        let mut src = CacheProxy::with_req_type::<str>(mock, None);

        let res = src.get_with_etag("abc").unwrap();

        assert_eq!(res, MockSrc::to_res("abc"));
        src.src.checkpoint();
    }

    #[test]
    fn test_get_repeat() {
        // get
        let mock = MockSrc::with_call_count(&CallCount {
            get_with_etag: Some(1),
            ..CallCount::zero()
        });
        let mut src = CacheProxy::with_req_type::<str>(mock, None);
        let res1 = src.get("abc").unwrap();
        src.src.checkpoint();

        src.src.setup(&CallCount {
            get_if_none_match: Some(2),
            ..CallCount::zero()
        });
        let res2 = src.get("abc").unwrap();
        let res3 = src.get("abc").unwrap();

        assert_eq!(res1, MockSrc::to_res("abc").data);
        assert_eq!(res2, MockSrc::to_res("abc").data);
        assert_eq!(res3, MockSrc::to_res("abc").data);
        src.src.checkpoint();

        // get_with_etag
        let mock = MockSrc::with_call_count(&CallCount {
            get_with_etag: Some(1),
            ..CallCount::zero()
        });
        let mut src = CacheProxy::with_req_type::<str>(mock, None);
        let res1 = src.get_with_etag("abc").unwrap();
        src.src.checkpoint();

        src.src.setup(&CallCount {
            get_if_none_match: Some(2),
            ..CallCount::zero()
        });
        let res2 = src.get_with_etag("abc").unwrap();
        let res3 = src.get_with_etag("abc").unwrap();

        assert_eq!(res1, MockSrc::to_res("abc"));
        assert_eq!(res2, MockSrc::to_res("abc"));
        assert_eq!(res3, MockSrc::to_res("abc"));
        src.src.checkpoint();

        // mixed. especially, cache is shared between get and get_with_etag.
        let mock = MockSrc::with_call_count(&CallCount {
            get_with_etag: Some(1),
            ..CallCount::zero()
        });
        let mut src = CacheProxy::with_req_type::<str>(mock, None);
        let res1 = src.get("abc").unwrap();
        src.src.checkpoint();

        src.src.setup(&CallCount {
            get_if_none_match: Some(2),
            ..CallCount::zero()
        });
        let res2 = src.get_with_etag("abc").unwrap();
        let res3 = src.get_with_etag("abc").unwrap();

        assert_eq!(res1, MockSrc::to_res("abc").data);
        assert_eq!(res2, MockSrc::to_res("abc"));
        assert_eq!(res3, MockSrc::to_res("abc"));
        src.src.checkpoint();

        //
        let mock = MockSrc::with_call_count(&CallCount {
            get_with_etag: Some(1),
            ..CallCount::zero()
        });
        let mut src = CacheProxy::with_req_type::<str>(mock, None);
        let res1 = src.get_with_etag("abc").unwrap();
        src.src.checkpoint();

        src.src.setup(&CallCount {
            get_if_none_match: Some(2),
            ..CallCount::zero()
        });
        let res2 = src.get("abc").unwrap();
        let res3 = src.get("abc").unwrap();

        assert_eq!(res1, MockSrc::to_res("abc"));
        assert_eq!(res2, MockSrc::to_res("abc").data);
        assert_eq!(res3, MockSrc::to_res("abc").data);
        src.src.checkpoint();
    }

    #[test]
    fn test_get_different() {
        // get
        let mock = MockSrc::with_call_count(&CallCount {
            get_with_etag: Some(1),
            ..CallCount::zero()
        });
        let mut src = CacheProxy::with_req_type::<str>(mock, None);
        let res1 = src.get("abc").unwrap();
        src.src.checkpoint();

        src.src.setup(&CallCount {
            get_with_etag: Some(1),
            get_if_none_match: Some(1),
            ..CallCount::zero()
        });
        let res2 = src.get("xyz").unwrap();
        let res3 = src.get("xyz").unwrap();
        src.src.checkpoint();

        assert_eq!(res1, MockSrc::to_res("abc").data);
        assert_eq!(res2, MockSrc::to_res("xyz").data);
        assert_eq!(res3, MockSrc::to_res("xyz").data);

        // get_with_etag
        let mock = MockSrc::with_call_count(&CallCount {
            get_with_etag: Some(1),
            ..CallCount::zero()
        });
        let mut src = CacheProxy::with_req_type::<str>(mock, None);
        let res1 = src.get_with_etag("abc").unwrap();
        src.src.checkpoint();

        src.src.setup(&CallCount {
            get_with_etag: Some(1),
            get_if_none_match: Some(1),
            ..CallCount::zero()
        });
        let res2 = src.get_with_etag("xyz").unwrap();
        let res3 = src.get_with_etag("xyz").unwrap();
        src.src.checkpoint();

        assert_eq!(res1, MockSrc::to_res("abc"));
        assert_eq!(res2, MockSrc::to_res("xyz"));
        assert_eq!(res3, MockSrc::to_res("xyz"));
    }

    #[test]
    fn test_get_after_modified() {
        let mock = MockSrc::with_call_count(&CallCount {
            get_with_etag: Some(1),
            ..CallCount::zero()
        });
        let mut src = CacheProxy::with_req_type::<str>(mock, None);
        let res1 = src.get_with_etag("abc").unwrap();
        assert_eq!(res1, MockSrc::to_res("abc"));
        src.src.checkpoint();
        // after modified source, get method of underlying is again called
        // even though cache is created. see `test_get_repeat`.
        // see `test_get_repeat` in which get method is called only once.
        src.src
            .expect_get_if_none_match()
            .once()
            .returning(|s, etag| {
                let res = MockSrc::to_res(&s.repeat(2));
                let res = if etag == res.etag { None } else { Some(res) };
                Ok(res)
            });

        let res2 = src.get_with_etag("abc").unwrap();

        assert_eq!(res2, MockSrc::to_res("abcabc"));
        src.src.checkpoint();
    }

    #[test]
    fn test_etag() {
        let mock = MockSrc::with_call_count(&CallCount {
            etag: Some(2),
            ..CallCount::zero()
        });
        let mut src = CacheProxy::with_req_type::<str>(mock, None);

        let etag1 = src.etag("abc").unwrap();
        let etag2 = src.etag("abc").unwrap();

        assert_eq!(etag1, MockSrc::to_res("abc").etag);
        assert_eq!(etag2, MockSrc::to_res("abc").etag);
        src.src.checkpoint();
    }

    #[rstest]
    #[case("abc", MockSrc::to_etag("abc"), true)]
    #[case("abc", MockSrc::to_etag("xyz"), false)]
    fn test_get_if_not_match(#[case] req: &str, #[case] etag: String, #[case] etag_match: bool) {
        let mock = MockSrc::with_call_count(&CallCount {
            get_if_none_match: Some(1),
            ..CallCount::zero()
        });
        let mut src = CacheProxy::with_req_type::<str>(mock, None);

        let res = src.get_if_none_match(req, &etag).unwrap();

        if etag_match {
            assert!(res.is_none());
        } else {
            assert_eq!(res, Some(MockSrc::to_res(req)));
        }
        src.src.checkpoint();
    }

    #[rstest]
    #[case(true)]
    #[case(false)]
    fn test_get_if_not_match_make_cache(#[case] etag_match: bool) {
        let mock = MockSrc::with_call_count(&CallCount {
            get_if_none_match: Some(1),
            ..CallCount::zero()
        });
        let mut src = CacheProxy::with_req_type::<str>(mock, None);
        let req = "abc";
        let etag = MockSrc::to_etag(if etag_match { "abc" } else { "xyz" });
        let _ = src.get_if_none_match(req, &etag).unwrap();
        src.src.checkpoint();
        if etag_match {
            // when get_if_not_match is called with matched etag,
            // nothing is cached because the underlying data source does not return anything.
            src.src.setup(&CallCount {
                get_with_etag: Some(1),
                ..CallCount::zero()
            });
        } else {
            // when get_if_not_match is called with unmatched etag,
            // the result is cached because the underlying data source returns something.
            src.src.setup(&CallCount {
                get_if_none_match: Some(1),
                ..CallCount::zero()
            });
        }

        let res = src.get(req).unwrap();

        assert_eq!(res, MockSrc::to_res(req).data);
        src.src.checkpoint();
    }

    #[test]
    fn test_propagate_err() {
        // get
        let mock = MockSrc::with_call_count(&CallCount {
            get_with_etag: Some(1),
            ..CallCount::zero()
        });
        let mut src = CacheProxy::with_req_type::<str>(mock, None);

        let res = src.get(ERR_REQ);

        assert_eq!(res.err().map(|e| e.to_string()).unwrap(), ERR_MSG);
        src.src.checkpoint();

        // get_with_etag
        let mock = MockSrc::with_call_count(&CallCount {
            get_with_etag: Some(1),
            ..CallCount::zero()
        });
        let mut src = CacheProxy::with_req_type::<str>(mock, None);

        let res = src.get_with_etag(ERR_REQ);

        assert_eq!(res.err().map(|e| e.to_string()).unwrap(), ERR_MSG);
        src.src.checkpoint();

        // etag
        let mock = MockSrc::with_call_count(&CallCount {
            etag: Some(1),
            ..CallCount::zero()
        });
        let mut src = CacheProxy::with_req_type::<str>(mock, None);

        let res = src.etag(ERR_REQ);

        assert_eq!(res.err().map(|e| e.to_string()).unwrap(), ERR_ETG_MSG);
        src.src.checkpoint();

        // get_if_none_match
        let mock = MockSrc::with_call_count(&CallCount {
            get_if_none_match: Some(1),
            ..CallCount::zero()
        });
        let mut src = CacheProxy::with_req_type::<str>(mock, None);

        let res = src.get_if_none_match(ERR_REQ, "");

        assert_eq!(res.err().map(|e| e.to_string()).unwrap(), ERR_MSG);
        src.src.checkpoint();
    }
}
