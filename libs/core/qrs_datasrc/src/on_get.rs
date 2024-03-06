use qrs_datasrc_derive::DebugTree;

use crate::{CacheableSrc, DataSrc, Response};

// -----------------------------------------------------------------------------
// OnGet
//

/// A data source with an action on request, such as logging.
///
/// # Example
/// ```
/// use std::sync::mpsc::channel;
/// use qrs_datasrc::{DataSrc, InMemory};
///
/// let data = {
///     let mut data = InMemory::new();
///     data.insert("a".to_owned(), 1);
///     data
/// };
///
/// let (tx, rx) = channel();
/// let data = InMemory::from(data)
///    .on_get(move |req, _| { tx.send(format!("requested: {req}")).unwrap(); });
///
/// let _ = data.get("a");
/// assert_eq!(rx.recv().unwrap(), "requested: a");
/// ```
#[derive(Debug, Clone, PartialEq, Eq, Hash, DebugTree)]
#[debug_tree(_use_from_qrs_datasrc, desc = "with action on get")]
pub struct OnGet<S, F> {
    #[debug_tree(subtree)]
    data_src: S,
    on_get: F,
}

//
// construction
//
impl<S: crate::DebugTree, F> OnGet<S, F> {
    #[inline]
    pub(super) fn new(data_src: S, on_get: F) -> Self {
        Self { data_src, on_get }
    }
}

//
// methods
//
impl<S, F> OnGet<S, F> {
    /// Get the inner data source.
    #[inline]
    pub fn inner(&self) -> &S {
        &self.data_src
    }

    /// Get the mutable reference to the inner data source.
    #[inline]
    pub fn inner_mut(&mut self) -> &mut S {
        &mut self.data_src
    }

    /// Unwrap the inner data source.
    #[inline]
    pub fn into_inner(self) -> S {
        self.data_src
    }
}

impl<S, F, Rq> DataSrc<Rq> for OnGet<S, F>
where
    Rq: ?Sized,
    S: DataSrc<Rq>,
    F: Fn(&Rq, Result<&S::Output, &anyhow::Error>),
{
    type Output = S::Output;

    #[inline]
    fn get(&self, req: &Rq) -> anyhow::Result<Self::Output> {
        let res = self.data_src.get(req);
        (self.on_get)(req, res.as_ref());
        res
    }
}

impl<S, F, Rq> CacheableSrc<Rq> for OnGet<S, F>
where
    Rq: ?Sized,
    S: CacheableSrc<Rq>,
    F: Fn(&Rq, Result<&S::Output, &anyhow::Error>),
{
    #[inline]
    fn etag(&self, req: &Rq) -> anyhow::Result<String> {
        self.data_src.etag(req)
    }

    #[inline]
    fn get_with_etag(&self, req: &Rq) -> anyhow::Result<Response<Self::Output>> {
        let res = self.data_src.get_with_etag(req);
        (self.on_get)(req, res.as_ref().map(|x| &x.data));
        res
    }

    #[inline]
    fn get_if_none_match(
        &self,
        req: &Rq,
        etag: &str,
    ) -> anyhow::Result<Option<Response<Self::Output>>> {
        let res = self.data_src.get_if_none_match(req, etag);
        match res.as_ref() {
            Ok(Some(res)) => (self.on_get)(req, Ok(&res.data)),
            Ok(None) => {}
            Err(e) => (self.on_get)(req, Err(e)),
        }
        res
    }
}

// =============================================================================
#[cfg(test)]
#[allow(clippy::type_complexity)]
mod tests {
    use std::sync::mpsc::{channel, Receiver};

    use super::*;
    use crate::*;
    use _test_util::*;
    use rstest::rstest;

    fn gen_action() -> (
        Receiver<(String, Result<String, String>)>,
        Box<dyn Fn(&str, Result<&String, &anyhow::Error>)>,
    ) {
        let (tx, rx) = channel();
        let action = move |arg: &str, res: Result<&String, &anyhow::Error>| {
            let res = res.map_err(|e| e.to_string()).map(|x| x.clone());
            tx.send((arg.to_owned(), res)).unwrap();
        };
        (rx, Box::new(action))
    }

    #[test]
    fn test_desc() {
        let (rx, action) = gen_action();
        let mock = MockSrc::with_call_count(&CallCount::zero());
        let mut src = OnGet::new(mock, action);

        let desc = src.desc();

        assert!(rx.try_recv().is_err());
        assert_eq!(desc, "with action on get");
        src.data_src.checkpoint();
    }

    #[test]
    fn test_debug_tree() {
        let (rx, action) = gen_action();
        let mock = MockSrc::with_call_count(&CallCount {
            debug_tree: Some(1),
            ..CallCount::zero()
        });
        let mut src = OnGet::new(mock, action);

        let tree = src.debug_tree();

        assert!(rx.try_recv().is_err());
        assert_eq!(
            tree,
            TreeInfo::Wrap {
                desc: "with action on get".to_owned(),
                tp: std::any::type_name::<
                    OnGet<MockSrc, Box<dyn Fn(&str, Result<&String, &anyhow::Error>)>>,
                >()
                .to_owned(),
                child: Box::new(TreeInfo::Leaf {
                    desc: MOCK_DESC.to_owned(),
                    tp: std::any::type_name::<MockSrc>().to_owned()
                })
            }
        );
        src.data_src.checkpoint();
    }

    #[rstest]
    #[case(true)]
    #[case(false)]
    fn test_get(#[case] err: bool) {
        let (rx, action) = gen_action();
        let mock = MockSrc::with_call_count(&CallCount {
            get: Some(1),
            ..CallCount::zero()
        });
        let req = if err { ERR_REQ } else { "abc" };
        let mut src = OnGet::new(mock, action);

        let res = src.get(req);

        if err {
            assert_eq!(res.unwrap_err().to_string(), ERR_MSG);
            assert_eq!(
                rx.recv().unwrap(),
                (req.to_owned(), Err(ERR_MSG.to_owned()))
            );
        } else {
            assert_eq!(res.unwrap(), MockSrc::to_res(req).data);
            assert_eq!(
                rx.recv().unwrap(),
                (req.to_owned(), Ok(MockSrc::to_res(req).data))
            );
        }
        src.data_src.checkpoint();
    }

    #[rstest]
    #[case(true)]
    #[case(false)]
    fn test_etag(#[case] err: bool) {
        let (rx, action) = gen_action();
        let mock = MockSrc::with_call_count(&CallCount {
            etag: Some(1),
            ..CallCount::zero()
        });
        let req = if err { ERR_REQ } else { "abc" };
        let mut src = OnGet::new(mock, action);

        let etag = src.etag(req);

        if err {
            assert_eq!(etag.unwrap_err().to_string(), ERR_ETG_MSG);
        } else {
            assert_eq!(etag.unwrap(), MockSrc::to_etag(req));
        }
        assert!(rx.try_recv().is_err());
        src.data_src.checkpoint();
    }

    #[rstest]
    #[case(true)]
    #[case(false)]
    fn test_get_with_etag(#[case] err: bool) {
        let (rx, action) = gen_action();
        let mock = MockSrc::with_call_count(&CallCount {
            get_with_etag: Some(1),
            ..CallCount::zero()
        });
        let req = if err { ERR_REQ } else { "abc" };
        let mut src = OnGet::new(mock, action);

        let res = src.get_with_etag(req);

        if err {
            assert_eq!(res.unwrap_err().to_string(), ERR_MSG);
            assert_eq!(
                rx.recv().unwrap(),
                (req.to_owned(), Err(ERR_MSG.to_owned()))
            );
        } else {
            assert_eq!(res.unwrap(), MockSrc::to_res(req));
            assert_eq!(
                rx.recv().unwrap(),
                (req.to_owned(), Ok(MockSrc::to_res(req).data))
            );
        }
        src.data_src.checkpoint();
    }

    #[rstest]
    #[case("match")]
    #[case("not_match")]
    #[case("err")]
    fn test_get_if_not_match(#[case] case: &str) {
        let (rx, action) = gen_action();
        let mock = MockSrc::with_call_count(&CallCount {
            get_if_none_match: Some(1),
            ..CallCount::zero()
        });
        let req = if case == "err" { ERR_REQ } else { "abc" };
        let etag = &MockSrc::to_etag(if case == "not_match" { "bar" } else { req });
        let mut src = OnGet::new(mock, action);

        let res = src.get_if_none_match(req, etag);

        if case == "match" {
            assert_eq!(res.unwrap(), None);
            assert!(rx.try_recv().is_err());
        } else if case == "not_match" {
            assert_eq!(res.unwrap(), Some(MockSrc::to_res(req)));
            assert_eq!(
                rx.recv().unwrap(),
                (req.to_owned(), Ok(MockSrc::to_res(req).data))
            );
        } else {
            assert_eq!(res.unwrap_err().to_string(), ERR_MSG);
            assert_eq!(
                rx.recv().unwrap(),
                (req.to_owned(), Err(ERR_MSG.to_owned()))
            );
        }
        src.data_src.checkpoint();
    }
}
