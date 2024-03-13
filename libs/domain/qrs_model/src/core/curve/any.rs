use std::fmt::Debug;

use dyn_clone::DynClone;

use super::YieldCurve;

// -----------------------------------------------------------------------------
// DynYieldCurve
//
pub trait DynYieldCurve<V>: YieldCurve<Value = V> + Debug + DynClone {}
impl<V, C> DynYieldCurve<V> for C where C: YieldCurve<Value = V> + Debug + DynClone {}

dyn_clone::clone_trait_object!(<V> DynYieldCurve<V>);
