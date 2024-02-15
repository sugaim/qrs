use std::sync::{Arc, Mutex};

use super::{node::DataSrc2Args, DataSrc, DataSrc3Args};

// -----------------------------------------------------------------------------
// TakeSnapshot
//

/// A data source that can take a snapshot of its current state.
pub trait TakeSnapshot: DataSrc {
    type SnapShot: DataSrc<Key = Self::Key, Output = Self::Output, Err = Self::Err>;
    type SnapShotErr;

    fn take_snapshot<'a, It>(&self, keys: It) -> Result<Self::SnapShot, Self::SnapShotErr>
    where
        It: IntoIterator<Item = &'a Self::Key>,
        Self::Key: 'a;
}

impl<T: TakeSnapshot> TakeSnapshot for Mutex<T> {
    type SnapShot = T::SnapShot;
    type SnapShotErr = T::SnapShotErr;

    fn take_snapshot<'a, It>(&self, keys: It) -> Result<Self::SnapShot, Self::SnapShotErr>
    where
        It: IntoIterator<Item = &'a Self::Key>,
        Self::Key: 'a,
    {
        self.lock().unwrap().take_snapshot(keys)
    }
}

impl<T: TakeSnapshot> TakeSnapshot for Arc<Mutex<T>> {
    type SnapShot = T::SnapShot;
    type SnapShotErr = T::SnapShotErr;

    fn take_snapshot<'a, It>(&self, keys: It) -> Result<Self::SnapShot, Self::SnapShotErr>
    where
        It: IntoIterator<Item = &'a Self::Key>,
        Self::Key: 'a,
    {
        self.lock().unwrap().take_snapshot(keys)
    }
}

// -----------------------------------------------------------------------------
// TakeSnapshot2Args
//

/// A data source that can take a snapshot of its current state.
pub trait TakeSnapshot2Args: DataSrc2Args {
    type SnapShot: DataSrc2Args<
        Key1 = Self::Key1,
        Key2 = Self::Key2,
        Output = Self::Output,
        Err = Self::Err,
    >;
    type SnapShotErr;

    fn take_snapshot<'a, It>(&self, keys: It) -> Result<Self::SnapShot, Self::SnapShotErr>
    where
        It: IntoIterator<Item = (&'a Self::Key1, &'a Self::Key2)>,
        Self::Key1: 'a,
        Self::Key2: 'a;
}

impl<T: TakeSnapshot2Args> TakeSnapshot2Args for Mutex<T> {
    type SnapShot = T::SnapShot;
    type SnapShotErr = T::SnapShotErr;

    fn take_snapshot<'a, It>(&self, keys: It) -> Result<Self::SnapShot, Self::SnapShotErr>
    where
        It: IntoIterator<Item = (&'a Self::Key1, &'a Self::Key2)>,
        Self::Key1: 'a,
        Self::Key2: 'a,
    {
        self.lock().unwrap().take_snapshot(keys)
    }
}

impl<T: TakeSnapshot2Args> TakeSnapshot2Args for Arc<Mutex<T>> {
    type SnapShot = T::SnapShot;
    type SnapShotErr = T::SnapShotErr;

    fn take_snapshot<'a, It>(&self, keys: It) -> Result<Self::SnapShot, Self::SnapShotErr>
    where
        It: IntoIterator<Item = (&'a Self::Key1, &'a Self::Key2)>,
        Self::Key1: 'a,
        Self::Key2: 'a,
    {
        self.lock().unwrap().take_snapshot(keys)
    }
}

// -----------------------------------------------------------------------------
// TakeSnapshot3Args
//

/// A data source that can take a snapshot of its current state.
pub trait TakeSnapshot3Args: DataSrc3Args {
    type SnapShot: DataSrc3Args<
        Key1 = Self::Key1,
        Key2 = Self::Key2,
        Key3 = Self::Key3,
        Output = Self::Output,
        Err = Self::Err,
    >;
    type SnapShotErr;

    fn take_snapshot<'a, It>(&self, keys: It) -> Result<Self::SnapShot, Self::SnapShotErr>
    where
        It: IntoIterator<Item = (&'a Self::Key1, &'a Self::Key2, &'a Self::Key3)>,
        Self::Key1: 'a,
        Self::Key2: 'a,
        Self::Key3: 'a;
}

impl<T: TakeSnapshot3Args> TakeSnapshot3Args for Mutex<T> {
    type SnapShot = T::SnapShot;
    type SnapShotErr = T::SnapShotErr;

    fn take_snapshot<'a, It>(&self, keys: It) -> Result<Self::SnapShot, Self::SnapShotErr>
    where
        It: IntoIterator<Item = (&'a Self::Key1, &'a Self::Key2, &'a Self::Key3)>,
        Self::Key1: 'a,
        Self::Key2: 'a,
        Self::Key3: 'a,
    {
        self.lock().unwrap().take_snapshot(keys)
    }
}

impl<T: TakeSnapshot3Args> TakeSnapshot3Args for Arc<Mutex<T>> {
    type SnapShot = T::SnapShot;
    type SnapShotErr = T::SnapShotErr;

    fn take_snapshot<'a, It>(&self, keys: It) -> Result<Self::SnapShot, Self::SnapShotErr>
    where
        It: IntoIterator<Item = (&'a Self::Key1, &'a Self::Key2, &'a Self::Key3)>,
        Self::Key1: 'a,
        Self::Key2: 'a,
        Self::Key3: 'a,
    {
        self.lock().unwrap().take_snapshot(keys)
    }
}
