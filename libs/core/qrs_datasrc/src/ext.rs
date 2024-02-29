mod on_change;
mod on_req;
mod snapshot;

use std::{
    any::Any,
    sync::{Arc, Mutex},
};

pub use on_req::OnReq;
pub use qrs_datasrc_derive::DebugTree;
pub use snapshot::{TakeSnapshot, TakeSnapshot2Args, TakeSnapshot3Args};

use crate::{DataSrc, DataSrc2Args, DataSrc3Args, StateId, Subject};

use self::on_change::_OnChange;

// -----------------------------------------------------------------------------
// SubjectExt
//
pub trait SubjectExt: Subject {
    /// Add an action, such as a logger, to be executed when the state of this subject changes.
    #[inline]
    #[must_use("The lifetime of the stored action is controlled by the returned value.")]
    fn on_change<F>(&mut self, action: F) -> Arc<dyn Any>
    where
        F: 'static + Send + Sync + Fn(&StateId),
    {
        let obs = Arc::new(Mutex::new(_OnChange::new(action)));
        self.reg_observer(Arc::downgrade(&obs) as _);
        obs as _
    }
}

impl<S: ?Sized + Subject> SubjectExt for S {}

// -----------------------------------------------------------------------------
// DataSrcExt
//
pub trait DataSrcExt: DataSrc {
    /// Add an action, such as a logger, to be executed when a request is made.
    #[inline]
    fn on_req<F>(self, action: F) -> OnReq<Self, F>
    where
        Self: Sized,
        F: Fn(&Self::Key, &Result<Self::Output, Self::Err>),
    {
        OnReq::new(self, action)
    }
}

impl<T: ?Sized + DataSrc> DataSrcExt for T {}

// -----------------------------------------------------------------------------
// DataSrc2ArgsExt
//
pub trait DataSrc2ArgsExt: DataSrc2Args {
    /// Add an action, such as a logger, to be executed when a request is made.
    #[inline]
    fn on_req<F>(self, action: F) -> OnReq<Self, F>
    where
        Self: Sized,
        F: Fn(&Self::Key1, &Self::Key2, &Result<Self::Output, Self::Err>),
    {
        OnReq::new(self, action)
    }
}

impl<T: ?Sized + DataSrc2Args> DataSrc2ArgsExt for T {}

// -----------------------------------------------------------------------------
// DataSrc3ArgsExt
//
pub trait DataSrc3ArgsExt: DataSrc3Args {
    /// Add an action, such as a logger, to be executed when a request is made.
    #[inline]
    fn on_req<F>(self, action: F) -> OnReq<Self, F>
    where
        Self: Sized,
        F: Fn(&Self::Key1, &Self::Key2, &Self::Key3, &Result<Self::Output, Self::Err>),
    {
        OnReq::new(self, action)
    }
}

impl<T: ?Sized + DataSrc3Args> DataSrc3ArgsExt for T {}
