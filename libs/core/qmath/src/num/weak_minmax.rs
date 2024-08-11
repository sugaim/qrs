// -----------------------------------------------------------------------------
// WeakMinMax
// -----------------------------------------------------------------------------
/// Relaxed version of [Ord::min] and [Ord::max] that returns
/// [None] if the values are incomparable.
pub trait WeakMinMax: Sized + PartialOrd {
    /// Returns the minimum of two values.
    #[inline]
    fn weak_min(self, other: Self) -> Option<Self> {
        match self.partial_cmp(&other) {
            Some(std::cmp::Ordering::Less) => Some(self),
            Some(_) => Some(other),
            None => None,
        }
    }

    /// Returns the maximum of two values.
    #[inline]
    fn weak_max(self, other: Self) -> Option<Self> {
        match self.partial_cmp(&other) {
            Some(std::cmp::Ordering::Greater) => Some(self),
            Some(_) => Some(other),
            None => None,
        }
    }

    /// Returns the minimum and maximum of two values.
    #[inline]
    fn weak_minmax(self, other: Self) -> Option<(Self, Self)> {
        match self.partial_cmp(&other) {
            Some(std::cmp::Ordering::Less) => Some((self, other)),
            Some(std::cmp::Ordering::Greater) => Some((other, self)),
            Some(std::cmp::Ordering::Equal) => Some((self, other)),
            None => None,
        }
    }
}

impl<T: PartialOrd> WeakMinMax for T {}

#[cfg(test)]
mod tests {
    use super::*;
    use rstest::rstest;

    #[rstest]
    #[case(1., 2., Some(1.), Some(2.))]
    #[case(1., 1., Some(1.), Some(1.))]
    #[case(f64::NAN, 1., None, None)]
    #[case(f64::INFINITY, 1., Some(1.), Some(f64::INFINITY))]
    #[case(f64::NEG_INFINITY, 1., Some(f64::NEG_INFINITY), Some(1.))]
    #[case(f64::NAN, f64::NAN, None, None)]
    #[case(f64::INFINITY, f64::NAN, None, None)]

    fn test_weak_minmax(
        #[case] lhs: f64,
        #[case] rhs: f64,
        #[case] expected_min: Option<f64>,
        #[case] expected_max: Option<f64>,
    ) {
        assert_eq!(lhs.weak_min(rhs), expected_min);
        assert_eq!(lhs.weak_max(rhs), expected_max);
        assert_eq!(rhs.weak_min(lhs), expected_min);
        assert_eq!(rhs.weak_max(lhs), expected_max);

        let (min, max) = match lhs.weak_minmax(rhs) {
            Some((min, max)) => (Some(min), Some(max)),
            None => (None, None),
        };
        assert_eq!(min, expected_min);
        assert_eq!(max, expected_max);
    }
}
