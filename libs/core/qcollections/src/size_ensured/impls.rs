// -----------------------------------------------------------------------------
// SplitFirst
// -----------------------------------------------------------------------------
pub struct SplitFirst<Item, Iter, const N: usize> {
    pub heads: [Item; N],
    pub tails: Iter,
}

impl<Item, Iter, const N: usize, H, T> From<SplitFirst<Item, Iter, N>> for (H, T)
where
    H: From<[Item; N]>,
    T: From<Iter>,
{
    #[inline]
    fn from(s: SplitFirst<Item, Iter, N>) -> Self {
        (s.heads.into(), s.tails.into())
    }
}

impl<Item, Iter, const N: usize> SplitFirst<Item, Iter, N> {
    #[inline]
    pub fn heads_into<T>(self) -> (T, Iter)
    where
        T: From<[Item; N]>,
    {
        (self.heads.into(), self.tails)
    }
    #[inline]
    pub fn tails_into<T>(self) -> ([Item; N], T)
    where
        T: From<Iter>,
    {
        (self.heads, self.tails.into())
    }
    #[inline]
    pub fn decompose(self) -> ([Item; N], Iter) {
        (self.heads, self.tails)
    }
}

// -----------------------------------------------------------------------------
// RefIntoIter
// -----------------------------------------------------------------------------
pub trait RefIntoIter<'this, ImplicitBound: sealed::Sealed = sealed::Bound<&'this Self>> {
    type Iter: ExactSizeIterator;
}

impl<'this, I> RefIntoIter<'this> for I
where
    &'this I: IntoIterator,
    <&'this I as IntoIterator>::IntoIter: ExactSizeIterator,
{
    type Iter = <&'this I as IntoIterator>::IntoIter;
}

// -----------------------------------------------------------------------------
// SizedContainer
// -----------------------------------------------------------------------------
pub trait SizedContainer: for<'this> RefIntoIter<'this> {
    fn iter(&self) -> <Self as RefIntoIter<'_>>::Iter;
    fn len(&self) -> usize {
        self.iter().len()
    }
}

impl<I> SizedContainer for I
where
    for<'this> &'this I: IntoIterator,
    for<'this> <&'this I as IntoIterator>::IntoIter: ExactSizeIterator,
{
    fn iter(&self) -> <Self as RefIntoIter<'_>>::Iter {
        self.into_iter()
    }
}

pub(super) mod sealed {
    pub trait Sealed {}
    pub struct Bound<T>(T);
    impl<T> Sealed for Bound<T> {}

    pub trait Has<const N: usize> {}

    macro_rules! impl_has {
        ($n:expr; $($m:expr),*) => {
            $(
                impl<T> Has<$m> for super::super::SizeEnsured<T, $n> {}
            )*
        };
    }

    impl_has!(1; 1);
    impl_has!(2; 1, 2);
    impl_has!(3; 1, 2, 3);
    impl_has!(4; 1, 2, 3, 4);
    impl_has!(5; 1, 2, 3, 4, 5);
    impl_has!(6; 1, 2, 3, 4, 5, 6);
    impl_has!(7; 1, 2, 3, 4, 5, 6, 7);
    impl_has!(8; 1, 2, 3, 4, 5, 6, 7, 8);
    impl_has!(9; 1, 2, 3, 4, 5, 6, 7, 8, 9);
    impl_has!(10; 1, 2, 3, 4, 5, 6, 7, 8, 9, 10);
    impl_has!(11; 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11);
    impl_has!(12; 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12);
}
