use std::sync::{Arc, Mutex};

use super::Subject;

// -----------------------------------------------------------------------------
// DataSrc
//
pub trait DataSrc: Subject {
    type Key: ?Sized;
    type Output;
    type Err;

    /// Request data with the given key
    /// In addition to the data, the state id of the node is also returned.
    ///
    /// - `Ok`: when the data is successfully retrieved
    /// - `Err`: when the data is not found or some error occurred
    fn req(&self, key: &Self::Key) -> Result<Self::Output, Self::Err>;
}

impl<T: ?Sized + DataSrc> DataSrc for Mutex<T> {
    type Key = T::Key;
    type Output = T::Output;
    type Err = T::Err;

    #[inline]
    fn req(&self, key: &Self::Key) -> Result<Self::Output, Self::Err> {
        self.lock().unwrap().req(key)
    }
}

impl<T: ?Sized + DataSrc> DataSrc for Arc<Mutex<T>> {
    type Key = T::Key;
    type Output = T::Output;
    type Err = T::Err;

    #[inline]
    fn req(&self, key: &Self::Key) -> Result<Self::Output, Self::Err> {
        self.lock().unwrap().req(key)
    }
}

// -----------------------------------------------------------------------------
// DataSrc2Args
//
pub trait DataSrc2Args: Subject {
    type Key1: ?Sized;
    type Key2: ?Sized;
    type Output;
    type Err;

    /// Request data with the given keys
    /// In addition to the data, the state id of the node is also returned.
    ///
    /// - `Ok`: when the data is successfully retrieved
    /// - `Err`: when the data is not found or some error occurred
    fn req(&self, key1: &Self::Key1, key2: &Self::Key2) -> Result<Self::Output, Self::Err>;
}

impl<T: ?Sized + DataSrc2Args> DataSrc2Args for Mutex<T> {
    type Key1 = T::Key1;
    type Key2 = T::Key2;
    type Output = T::Output;
    type Err = T::Err;

    #[inline]
    fn req(&self, key1: &Self::Key1, key2: &Self::Key2) -> Result<Self::Output, Self::Err> {
        self.lock().unwrap().req(key1, key2)
    }
}

impl<T: ?Sized + DataSrc2Args> DataSrc2Args for Arc<Mutex<T>> {
    type Key1 = T::Key1;
    type Key2 = T::Key2;
    type Output = T::Output;
    type Err = T::Err;

    #[inline]
    fn req(&self, key1: &Self::Key1, key2: &Self::Key2) -> Result<Self::Output, Self::Err> {
        self.lock().unwrap().req(key1, key2)
    }
}

// -----------------------------------------------------------------------------
// DataSrc3Args
//
pub trait DataSrc3Args: Subject {
    type Key1: ?Sized;
    type Key2: ?Sized;
    type Key3: ?Sized;
    type Output;
    type Err;

    /// Request data with the given keys
    /// In addition to the data, the state id of the node is also returned.
    ///
    /// - `Ok`: when the data is successfully retrieved
    /// - `Err`: when the data is not found or some error occurred
    fn req(
        &self,
        key1: &Self::Key1,
        key2: &Self::Key2,
        key3: &Self::Key3,
    ) -> Result<Self::Output, Self::Err>;
}

impl<T: ?Sized + DataSrc3Args> DataSrc3Args for Mutex<T> {
    type Key1 = T::Key1;
    type Key2 = T::Key2;
    type Key3 = T::Key3;
    type Output = T::Output;
    type Err = T::Err;

    #[inline]
    fn req(
        &self,
        key1: &Self::Key1,
        key2: &Self::Key2,
        key3: &Self::Key3,
    ) -> Result<Self::Output, Self::Err> {
        self.lock().unwrap().req(key1, key2, key3)
    }
}

impl<T: ?Sized + DataSrc3Args> DataSrc3Args for Arc<Mutex<T>> {
    type Key1 = T::Key1;
    type Key2 = T::Key2;
    type Key3 = T::Key3;
    type Output = T::Output;
    type Err = T::Err;

    #[inline]
    fn req(
        &self,
        key1: &Self::Key1,
        key2: &Self::Key2,
        key3: &Self::Key3,
    ) -> Result<Self::Output, Self::Err> {
        self.lock().unwrap().req(key1, key2, key3)
    }
}
