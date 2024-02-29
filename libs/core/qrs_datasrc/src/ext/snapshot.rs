use std::sync::{Arc, Mutex};

use crate::{DataSrc, DataSrc2Args, DataSrc3Args};

// -----------------------------------------------------------------------------
// TakeSnapshot
//
pub trait TakeSnapshot: DataSrc {
    type Snapshot: DataSrc<Key = Self::Key, Output = Self::Output, Err = Self::Err>;
    type SnapshotErr;

    fn take_snapshot<'a, It>(&self, it: It) -> Result<Self::Snapshot, Self::SnapshotErr>
    where
        It: IntoIterator<Item = &'a Self::Key>,
        Self::Key: 'a;
}

impl<T> TakeSnapshot for Mutex<T>
where
    T: TakeSnapshot,
{
    type Snapshot = T::Snapshot;
    type SnapshotErr = T::SnapshotErr;

    #[inline]
    fn take_snapshot<'a, It>(&self, it: It) -> Result<Self::Snapshot, Self::SnapshotErr>
    where
        It: IntoIterator<Item = &'a Self::Key>,
        Self::Key: 'a,
    {
        self.lock().unwrap().take_snapshot(it)
    }
}

impl<T> TakeSnapshot for Arc<Mutex<T>>
where
    T: TakeSnapshot,
{
    type Snapshot = T::Snapshot;
    type SnapshotErr = T::SnapshotErr;

    #[inline]
    fn take_snapshot<'a, It>(&self, it: It) -> Result<Self::Snapshot, Self::SnapshotErr>
    where
        It: IntoIterator<Item = &'a Self::Key>,
        Self::Key: 'a,
    {
        self.lock().unwrap().take_snapshot(it)
    }
}

// -----------------------------------------------------------------------------
// TakeSnapshot2Args
//
pub trait TakeSnapshot2Args: DataSrc2Args {
    type Snapshot: DataSrc2Args<
        Key1 = Self::Key1,
        Key2 = Self::Key2,
        Output = Self::Output,
        Err = Self::Err,
    >;
    type SnapshotErr;

    fn take_snapshot<'a, It, It2>(&self, it: It) -> Result<Self::Snapshot, Self::SnapshotErr>
    where
        It: IntoIterator<Item = (&'a Self::Key1, It2)>,
        It2: IntoIterator<Item = &'a Self::Key2>,
        Self::Key1: 'a,
        Self::Key2: 'a;
}

impl<T> TakeSnapshot2Args for Mutex<T>
where
    T: TakeSnapshot2Args,
{
    type Snapshot = T::Snapshot;
    type SnapshotErr = T::SnapshotErr;

    #[inline]
    fn take_snapshot<'a, It, It2>(&self, it: It) -> Result<Self::Snapshot, Self::SnapshotErr>
    where
        It: IntoIterator<Item = (&'a Self::Key1, It2)>,
        It2: IntoIterator<Item = &'a Self::Key2>,
        Self::Key1: 'a,
        Self::Key2: 'a,
    {
        self.lock().unwrap().take_snapshot(it)
    }
}

impl<T> TakeSnapshot2Args for Arc<Mutex<T>>
where
    T: TakeSnapshot2Args,
{
    type Snapshot = T::Snapshot;
    type SnapshotErr = T::SnapshotErr;

    #[inline]
    fn take_snapshot<'a, It, It2>(&self, it: It) -> Result<Self::Snapshot, Self::SnapshotErr>
    where
        It: IntoIterator<Item = (&'a Self::Key1, It2)>,
        It2: IntoIterator<Item = &'a Self::Key2>,
        Self::Key1: 'a,
        Self::Key2: 'a,
    {
        self.lock().unwrap().take_snapshot(it)
    }
}

// -----------------------------------------------------------------------------
// TakeSnapshot3Args
//
pub trait TakeSnapshot3Args: DataSrc3Args {
    type Snapshot: DataSrc3Args<
        Key1 = Self::Key1,
        Key2 = Self::Key2,
        Key3 = Self::Key3,
        Output = Self::Output,
        Err = Self::Err,
    >;
    type SnapshotErr;

    fn take_snapshot<'a, It, It2, It3>(&self, it: It) -> Result<Self::Snapshot, Self::SnapshotErr>
    where
        It: IntoIterator<Item = (&'a Self::Key1, It2)>,
        It2: IntoIterator<Item = (&'a Self::Key2, It3)>,
        It3: IntoIterator<Item = &'a Self::Key3>,
        Self::Key1: 'a,
        Self::Key2: 'a,
        Self::Key3: 'a;
}

impl<T> TakeSnapshot3Args for Mutex<T>
where
    T: TakeSnapshot3Args,
{
    type Snapshot = T::Snapshot;
    type SnapshotErr = T::SnapshotErr;

    #[inline]
    fn take_snapshot<'a, It, It2, It3>(&self, it: It) -> Result<Self::Snapshot, Self::SnapshotErr>
    where
        It: IntoIterator<Item = (&'a Self::Key1, It2)>,
        It2: IntoIterator<Item = (&'a Self::Key2, It3)>,
        It3: IntoIterator<Item = &'a Self::Key3>,
        Self::Key1: 'a,
        Self::Key2: 'a,
        Self::Key3: 'a,
    {
        self.lock().unwrap().take_snapshot(it)
    }
}

impl<T> TakeSnapshot3Args for Arc<Mutex<T>>
where
    T: TakeSnapshot3Args,
{
    type Snapshot = T::Snapshot;
    type SnapshotErr = T::SnapshotErr;

    #[inline]
    fn take_snapshot<'a, It, It2, It3>(&self, it: It) -> Result<Self::Snapshot, Self::SnapshotErr>
    where
        It: IntoIterator<Item = (&'a Self::Key1, It2)>,
        It2: IntoIterator<Item = (&'a Self::Key2, It3)>,
        It3: IntoIterator<Item = &'a Self::Key3>,
        Self::Key1: 'a,
        Self::Key2: 'a,
        Self::Key3: 'a,
    {
        self.lock().unwrap().take_snapshot(it)
    }
}
