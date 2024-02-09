use super::DataSrc;

// -----------------------------------------------------------------------------
// TakeSnapshot
//

/// A data source that can take a snapshot of its current state.
pub trait TakeSnapshot<K: ?Sized>: DataSrc<K> {
    type SnapShot: DataSrc<K, Output = Self::Output>;

    fn take_snapshot<'a, It>(&self, keys: It) -> Result<Self::SnapShot, Self::Err>
    where
        It: IntoIterator<Item = &'a K>,
        K: 'a;
}
