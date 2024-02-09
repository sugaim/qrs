use super::{node::DataSrc2Args, DataSrc, DataSrc3Args};

// -----------------------------------------------------------------------------
// TakeSnapshot
//

/// A data source that can take a snapshot of its current state.
pub trait TakeSnapshot<K: ?Sized>: DataSrc<K> {
    type SnapShot: DataSrc<K, Output = Self::Output, Err = Self::Err>;
    type SnapShotErr;

    fn take_snapshot<'a, It>(&self, keys: It) -> Result<Self::SnapShot, Self::SnapShotErr>
    where
        It: IntoIterator<Item = &'a K>,
        K: 'a;
}

// -----------------------------------------------------------------------------
// TakeSnapshot2Args
//

/// A data source that can take a snapshot of its current state.
pub trait TakeSnapshot2Args<K1: ?Sized, K2: ?Sized>: DataSrc2Args<K1, K2> {
    type SnapShot: DataSrc2Args<K1, K2, Output = Self::Output, Err = Self::Err>;
    type SnapShotErr;

    fn take_snapshot<'a, It>(&self, keys: It) -> Result<Self::SnapShot, Self::SnapShotErr>
    where
        It: IntoIterator<Item = (&'a K1, &'a K2)>,
        K1: 'a,
        K2: 'a;
}

// -----------------------------------------------------------------------------
// TakeSnapshot3Args
//

/// A data source that can take a snapshot of its current state.
pub trait TakeSnapshot3Args<K1: ?Sized, K2: ?Sized, K3: ?Sized>: DataSrc3Args<K1, K2, K3> {
    type SnapShot: DataSrc3Args<K1, K2, K3, Output = Self::Output, Err = Self::Err>;
    type SnapShotErr;

    fn take_snapshot<'a, It>(&self, keys: It) -> Result<Self::SnapShot, Self::SnapShotErr>
    where
        It: IntoIterator<Item = (&'a K1, &'a K2, &'a K3)>,
        K1: 'a,
        K2: 'a,
        K3: 'a;
}
