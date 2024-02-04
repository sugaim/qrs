use std::cmp::Ordering;

/// Trait to define total ordering even for floating point numbers.
///
/// Sorting values are sometimes useful for reducing the computational cost.
/// However, floating point numbers cannot be sorted by the default `Ord` trait.
/// Hence, this trait provides a total ordering for floating point numbers to sort them.
///
/// Note that comparison provided by this trait is for sorting only.
/// Especially, the result of this comparison does not always match the result of `PartialOrd` trait.
///
/// # Examples
/// ```
/// use std::cmp::Ordering;
/// use qcore::math::num::TotalCmpForSort;
///
/// // ordinary comparison
/// assert_eq!(TotalCmpForSort::total_cmp_for_sort(&1.0, &2.0), Ordering::Less);
/// assert_eq!(TotalCmpForSort::total_cmp_for_sort(&2.0, &1.0), Ordering::Greater);
/// assert_eq!(TotalCmpForSort::total_cmp_for_sort(&1.0, &1.0), Ordering::Equal);
///
/// // unusual comparison (as example)
/// assert_ne!(TotalCmpForSort::total_cmp_for_sort(&-0.0, &0.0), (-0.0).partial_cmp(&0.0).unwrap());
/// ```
pub trait TotalCmpForSort {
    fn total_cmp_for_sort(&self, other: &Self) -> Ordering;
}

macro_rules! define_total_cmp_for_int {
    ($($t:ty),*) => {
        $(
            impl TotalCmpForSort for $t {
                fn total_cmp_for_sort(&self, other: &Self) -> Ordering {
                    self.cmp(other)
                }
            }
        )*
    };
}

define_total_cmp_for_int!(i8, i16, i32, i64, i128, isize, u8, u16, u32, u64, u128, usize);

impl TotalCmpForSort for f32 {
    /// Comparison defined by IEEE 754.
    /// See [this page](https://doc.rust-lang.org/std/primitive.f32.html#method.total_cmp) for details.
    fn total_cmp_for_sort(&self, other: &Self) -> Ordering {
        f32::total_cmp(&self, other)
    }
}

impl TotalCmpForSort for f64 {
    /// Comparison defined by IEEE 754.
    /// See [this page](https://doc.rust-lang.org/std/primitive.f64.html#method.total_cmp) for details.
    fn total_cmp_for_sort(&self, other: &Self) -> Ordering {
        f64::total_cmp(&self, other)
    }
}
