// -----------------------------------------------------------------------------
// PartialOrdMinMax
//
pub trait PartialOrdMinMax: Sized + PartialOrd {
    /// Returns the minimum of two values.
    fn partial_ord_min(self, other: Self) -> Option<Self> {
        match self.partial_cmp(&other) {
            Some(std::cmp::Ordering::Less) => Some(self),
            Some(_) => Some(other),
            None => None,
        }
    }

    /// Returns the maximum of two values.
    fn partial_ord_max(self, other: Self) -> Option<Self> {
        match self.partial_cmp(&other) {
            Some(std::cmp::Ordering::Greater) => Some(self),
            Some(_) => Some(other),
            None => None,
        }
    }
}

impl<T: PartialOrd> PartialOrdMinMax for T {}
