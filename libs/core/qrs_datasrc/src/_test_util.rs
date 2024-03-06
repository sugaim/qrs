use crate::*;

pub const MOCK_DESC: &str = "mock";
pub const ERR_REQ: &str = "err";
pub const ERR_MSG: &str = "error";
pub const ERR_ETG_MSG: &str = "etag-error";

mockall::mock! {
    pub Src {}

    impl DebugTree for Src {
        fn desc(&self) -> String;
        fn debug_tree(&self) -> TreeInfo;
    }

    impl DataSrc<str> for Src {
        type Output = String;
        fn get(&self, req: &str) -> anyhow::Result<String>;
    }

    impl CacheableSrc<str> for Src {
        fn etag(&self, req: &str) -> anyhow::Result<String>;
        fn get_with_etag(&self, req: &str) -> anyhow::Result<Response<String>>;
        fn get_if_none_match(
            &self,
            req: &str,
            etag: &str,
        ) -> anyhow::Result<Option<Response<String>>>;
    }
}

#[derive(Default)]
pub struct CallCount {
    pub desc: Option<usize>,
    pub debug_tree: Option<usize>,
    pub get: Option<usize>,
    pub etag: Option<usize>,
    pub get_with_etag: Option<usize>,
    pub get_if_none_match: Option<usize>,
}

impl CallCount {
    pub fn zero() -> Self {
        CallCount {
            desc: Some(0),
            debug_tree: Some(0),
            get: Some(0),
            etag: Some(0),
            get_with_etag: Some(0),
            get_if_none_match: Some(0),
        }
    }
}

impl MockSrc {
    pub fn to_etag(s: &str) -> String {
        format!("etag-{}", s)
    }

    pub fn to_res(s: &str) -> Response<String> {
        Response {
            data: s.chars().rev().collect(),
            etag: format!("etag-{}", s),
        }
    }

    pub fn with_call_count(cnt: &CallCount) -> Self {
        let mut mock = MockSrc::new();
        mock.setup(cnt);
        mock
    }
    pub fn setup(&mut self, cnt: &CallCount) {
        //
        let desc = self.expect_desc().returning(|| MOCK_DESC.to_owned());
        if let Some(cnt) = cnt.desc {
            desc.times(cnt);
        }

        //
        let debug_tree = self.expect_debug_tree().returning(|| TreeInfo::Leaf {
            desc: MOCK_DESC.to_owned(),
            tp: std::any::type_name::<Self>().to_owned(),
        });
        if let Some(cnt) = cnt.debug_tree {
            debug_tree.times(cnt);
        }

        //
        let get = self.expect_get().returning(|s| {
            if s == ERR_REQ {
                Err(anyhow::anyhow!(ERR_MSG))
            } else {
                Ok(Self::to_res(s).data)
            }
        });
        if let Some(cnt) = cnt.get {
            get.times(cnt);
        }

        //
        let etag = self.expect_etag().returning(|s| {
            if s == ERR_REQ {
                Err(anyhow::anyhow!(ERR_ETG_MSG))
            } else {
                Ok(Self::to_res(s).etag)
            }
        });
        if let Some(cnt) = cnt.etag {
            etag.times(cnt);
        }

        //
        let get_with_etag = self.expect_get_with_etag().returning(|s| {
            if s == ERR_REQ {
                Err(anyhow::anyhow!(ERR_MSG))
            } else {
                Ok(Self::to_res(s))
            }
        });
        if let Some(cnt) = cnt.get_with_etag {
            get_with_etag.times(cnt);
        }

        //
        let get_if_none_match = self.expect_get_if_none_match().returning(|s, etag| {
            if s == ERR_REQ {
                Err(anyhow::anyhow!(ERR_MSG))
            } else if etag == Self::to_res(s).etag {
                Ok(None)
            } else {
                Ok(Some(Self::to_res(s)))
            }
        });
        if let Some(cnt) = cnt.get_if_none_match {
            get_if_none_match.times(cnt);
        }
    }
}
