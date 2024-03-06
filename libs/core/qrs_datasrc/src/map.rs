use qrs_datasrc_derive::DebugTree;

use crate::{CacheableSrc, DataSrc, Response};

// -----------------------------------------------------------------------------
// StatelessFunc
//
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct StatelessFunc<F>(F);

// -----------------------------------------------------------------------------
// Map
//
/// Map the output of the underlying data source
///
/// This data source maps the output of the underlying data source using the given
/// function.
/// Note that created object implements [`crate::CacheableSrc`]
/// only when the mapping function is `fn` because
/// stateful function may lead to inconsistent etag.
#[derive(Clone, Debug, PartialEq, Eq, Hash, DebugTree)]
#[debug_tree(_use_from_qrs_datasrc, desc = "map")]
pub struct Map<Src, F> {
    #[debug_tree(subtree)]
    src: Src,
    map: F,
}

//
// construction
//
impl<Src, F> Map<Src, F> {
    #[inline]
    pub(super) fn new(src: Src, map: F) -> Self {
        Self { src, map }
    }
}

//
// methods
//
impl<Src, F> Map<Src, F> {
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

    /// Ensure the mapping function is stateless.
    #[inline]
    pub fn ensure_stateless_func(self) -> Map<Src, StatelessFunc<F>> {
        Map::new(self.src, StatelessFunc(self.map))
    }
}

impl<S, F, O, Rq> DataSrc<Rq> for Map<S, F>
where
    Rq: ?Sized,
    S: DataSrc<Rq>,
    F: Fn(S::Output) -> anyhow::Result<O>,
{
    type Output = O;

    #[inline]
    fn get(&self, req: &Rq) -> anyhow::Result<Self::Output> {
        self.src.get(req).and_then(|r| (self.map)(r))
    }
}

impl<S, F, O, Rq> DataSrc<Rq> for Map<S, StatelessFunc<F>>
where
    Rq: ?Sized,
    S: DataSrc<Rq>,
    F: Fn(S::Output) -> anyhow::Result<O>,
{
    type Output = O;

    #[inline]
    fn get(&self, req: &Rq) -> anyhow::Result<Self::Output> {
        self.src.get(req).and_then(|r| (self.map.0)(r))
    }
}

impl<S, O, Rq> CacheableSrc<Rq> for Map<S, fn(S::Output) -> anyhow::Result<O>>
where
    Rq: ?Sized,
    S: CacheableSrc<Rq>,
{
    #[inline]
    fn etag(&self, req: &Rq) -> anyhow::Result<String> {
        self.src.etag(req)
    }

    #[inline]
    fn get_with_etag(&self, req: &Rq) -> anyhow::Result<Response<Self::Output>> {
        let Response { data, etag } = self.src.get_with_etag(req)?;
        let data = (self.map)(data)?;
        Ok(Response { data, etag })
    }

    #[inline]
    fn get_if_none_match(
        &self,
        req: &Rq,
        etag: &str,
    ) -> anyhow::Result<Option<Response<Self::Output>>> {
        match self.src.get_if_none_match(req, etag)? {
            None => Ok(None),
            Some(Response { data, etag }) => {
                let data = (self.map)(data)?;
                Ok(Some(Response { data, etag }))
            }
        }
    }
}

impl<S, O, F, Rq> CacheableSrc<Rq> for Map<S, StatelessFunc<F>>
where
    Rq: ?Sized,
    S: CacheableSrc<Rq>,
    F: Fn(S::Output) -> anyhow::Result<O>,
{
    #[inline]
    fn etag(&self, req: &Rq) -> anyhow::Result<String> {
        self.src.etag(req)
    }

    #[inline]
    fn get_with_etag(&self, req: &Rq) -> anyhow::Result<Response<Self::Output>> {
        let Response { data, etag } = self.src.get_with_etag(req)?;
        let data = (self.map.0)(data)?;
        Ok(Response { data, etag })
    }

    #[inline]
    fn get_if_none_match(
        &self,
        req: &Rq,
        etag: &str,
    ) -> anyhow::Result<Option<Response<Self::Output>>> {
        match self.src.get_if_none_match(req, etag)? {
            None => Ok(None),
            Some(Response { data, etag }) => {
                let data = (self.map.0)(data)?;
                Ok(Some(Response { data, etag }))
            }
        }
    }
}

// =============================================================================
#[cfg(test)]
mod tests {
    use super::*;
    use crate::*;

    use _test_util::*;
    use anyhow::anyhow;
    use rstest::rstest;

    fn mapping() -> fn(String) -> anyhow::Result<String> {
        |res| {
            if res == ERR_REQ {
                Err(anyhow!("map: {}", ERR_MSG))
            } else {
                Ok(res.to_uppercase())
            }
        }
    }

    #[rstest]
    #[case("succ")]
    #[case("base_err")]
    #[case("map_err")]
    fn test_get(#[case] case: &str) {
        let req = if case == "base_err" {
            ERR_REQ.to_owned()
        } else if case == "map_err" {
            ERR_REQ.chars().rev().collect() // Mock will return reversed string
        } else {
            "abc".to_owned()
        };

        let check = |res: anyhow::Result<String>| {
            if case == "base_err" {
                assert_eq!(res.unwrap_err().to_string(), ERR_MSG);
            } else if case == "map_err" {
                assert_eq!(res.unwrap_err().to_string(), format!("map: {}", ERR_MSG));
            } else {
                assert_eq!(res.unwrap(), MockSrc::to_res(&req).data.to_uppercase());
            }
        };

        // ordinary
        let mock = MockSrc::with_call_count(&CallCount {
            get: Some(1),
            ..CallCount::zero()
        });
        let mut src = Map::new(mock, mapping());

        let res = src.get(&req);

        check(res);
        src.src.checkpoint();

        // force stateless
        let mock = MockSrc::with_call_count(&CallCount {
            get: Some(1),
            ..CallCount::zero()
        });
        let mut src = Map::new(mock, mapping()).ensure_stateless_func();

        let res = src.get(&req);

        check(res);
        src.src.checkpoint();
    }

    #[rstest]
    #[case(true)]
    #[case(false)]
    fn test_etag(#[case] err: bool) {
        let req = if err { ERR_REQ } else { "abc" };
        let check = |res: anyhow::Result<String>| {
            if err {
                assert_eq!(res.unwrap_err().to_string(), ERR_ETG_MSG);
            } else {
                assert_eq!(res.unwrap(), MockSrc::to_etag(req));
            }
        };

        // ordinary
        let mock = MockSrc::with_call_count(&CallCount {
            etag: Some(1),
            ..CallCount::zero()
        });
        let mut src = Map::new(mock, mapping());

        let res = src.etag(req);

        check(res);
        src.src.checkpoint();

        // force stateless
        let mock = MockSrc::with_call_count(&CallCount {
            etag: Some(1),
            ..CallCount::zero()
        });
        let mut src = Map::new(mock, mapping()).ensure_stateless_func();

        let res = src.etag(req);

        check(res);
        src.src.checkpoint();
    }

    #[rstest]
    #[case("succ")]
    #[case("base_err")]
    #[case("map_err")]
    fn test_get_with_etag(#[case] case: &str) {
        let req = if case == "base_err" {
            ERR_REQ.to_owned()
        } else if case == "map_err" {
            ERR_REQ.chars().rev().collect() // Mock will return reversed string
        } else {
            "abc".to_owned()
        };
        let check = |res: anyhow::Result<Response<String>>| {
            if case == "base_err" {
                assert_eq!(res.unwrap_err().to_string(), ERR_MSG);
            } else if case == "map_err" {
                assert_eq!(res.unwrap_err().to_string(), format!("map: {}", ERR_MSG));
            } else {
                assert_eq!(
                    res.as_ref().unwrap().data,
                    MockSrc::to_res(&req).data.to_uppercase()
                );
                assert_eq!(res.as_ref().unwrap().etag, MockSrc::to_res(&req).etag);
            }
        };

        // ordinary
        let mock = MockSrc::with_call_count(&CallCount {
            get_with_etag: Some(1),
            ..CallCount::zero()
        });

        let mut src = Map::new(mock, mapping());

        let res = src.get_with_etag(&req);

        check(res);
        src.src.checkpoint();

        // force stateless
        let mock = MockSrc::with_call_count(&CallCount {
            get_with_etag: Some(1),
            ..CallCount::zero()
        });
        let mut src = Map::new(mock, mapping());

        let res = src.get_with_etag(&req);

        check(res);
        src.src.checkpoint();
    }

    #[rstest]
    #[case("match")]
    #[case("not_match")]
    #[case("base_err")]
    #[case("map_err")]
    fn test_get_if_not_match(#[case] case: &str) {
        let (req, etag) = if case == "base_err" {
            (ERR_REQ.to_owned(), MockSrc::to_etag(ERR_REQ))
        } else if case == "map_err" {
            (ERR_REQ.chars().rev().collect(), MockSrc::to_etag("abc"))
        } else if case == "match" {
            ("abc".to_owned(), MockSrc::to_etag("abc"))
        } else {
            ("abc".to_owned(), MockSrc::to_etag("xyz"))
        };
        let check = |res: anyhow::Result<Option<Response<String>>>| {
            if case == "base_err" {
                assert_eq!(res.unwrap_err().to_string(), ERR_MSG);
            } else if case == "map_err" {
                assert_eq!(res.unwrap_err().to_string(), format!("map: {}", ERR_MSG));
            } else if case == "match" {
                assert!(res.unwrap().is_none());
            } else {
                assert_eq!(
                    res.as_ref().unwrap().as_ref().unwrap().data,
                    MockSrc::to_res(&req).data.to_uppercase()
                );
                assert_eq!(
                    res.as_ref().unwrap().as_ref().unwrap().etag,
                    MockSrc::to_res(&req).etag
                );
            }
        };

        // ordinary
        let mock = MockSrc::with_call_count(&CallCount {
            get_if_none_match: Some(1),
            ..CallCount::zero()
        });
        let mut src = Map::new(mock, mapping());

        let res = src.get_if_none_match(&req, &etag);

        check(res);
        src.src.checkpoint();

        // force stateless
        let mock = MockSrc::with_call_count(&CallCount {
            get_if_none_match: Some(1),
            ..CallCount::zero()
        });
        let mut src = Map::new(mock, mapping()).ensure_stateless_func();

        let res = src.get_if_none_match(&req, &etag);

        check(res);
        src.src.checkpoint();
    }
}
