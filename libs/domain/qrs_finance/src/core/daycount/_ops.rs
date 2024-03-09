macro_rules! define_vector_behavior {
    ($generic_rate:ident) => {
        impl<V: qrs_math::num::FloatBased> qrs_math::num::FloatBased for $generic_rate<V> {
            type BaseFloat = V::BaseFloat;
        }

        impl<V: qrs_math::num::Arithmetic> qrs_math::num::Zero for $generic_rate<V> {
            #[inline]
            fn zero() -> Self {
                Self(V::zero())
            }

            #[inline]
            fn is_zero(&self) -> bool {
                self.0.is_zero()
            }
        }

        impl<V: qrs_math::num::Arithmetic> std::ops::Neg for $generic_rate<V> {
            type Output = Self;

            #[inline]
            fn neg(self) -> Self::Output {
                Self(-self.0)
            }
        }

        impl<V: qrs_math::num::Arithmetic> std::ops::Add for $generic_rate<V> {
            type Output = Self;

            #[inline]
            fn add(self, rhs: Self) -> Self::Output {
                Self(self.0 + rhs.0)
            }
        }

        impl<V: qrs_math::num::Arithmetic> std::ops::Add<&Self> for $generic_rate<V> {
            type Output = Self;

            #[inline]
            fn add(self, rhs: &Self) -> Self::Output {
                Self(self.0 + &rhs.0)
            }
        }

        impl<V: qrs_math::num::Arithmetic> std::ops::AddAssign<&Self> for $generic_rate<V> {
            #[inline]
            fn add_assign(&mut self, rhs: &Self) {
                self.0 += &rhs.0;
            }
        }

        impl<V: qrs_math::num::Arithmetic> std::ops::Sub<&Self> for $generic_rate<V> {
            type Output = Self;

            #[inline]
            fn sub(self, rhs: &Self) -> Self::Output {
                Self(self.0 - &rhs.0)
            }
        }

        impl<V: qrs_math::num::Arithmetic> std::ops::SubAssign<&Self> for $generic_rate<V> {
            #[inline]
            fn sub_assign(&mut self, rhs: &Self) {
                self.0 -= &rhs.0;
            }
        }

        impl<K, V> std::ops::Mul<&K> for $generic_rate<V>
        where
            V: qrs_math::num::FloatBased
                + qrs_math::num::Vector<V::BaseFloat>
                + for<'a> Mul<&'a K, Output = V>,
        {
            type Output = $generic_rate<V>;

            #[inline]
            fn mul(self, rhs: &K) -> Self::Output {
                Self(self.0 * rhs)
            }
        }

        impl<K, V> std::ops::MulAssign<&K> for $generic_rate<V>
        where
            V: qrs_math::num::FloatBased
                + qrs_math::num::Vector<V::BaseFloat>
                + for<'a> MulAssign<&'a K>,
        {
            #[inline]
            fn mul_assign(&mut self, rhs: &K) {
                self.0 *= rhs;
            }
        }

        impl<K, V> std::ops::Div<&K> for $generic_rate<V>
        where
            V: qrs_math::num::FloatBased
                + qrs_math::num::Vector<V::BaseFloat>
                + for<'a> Div<&'a K, Output = V>,
        {
            type Output = $generic_rate<V>;

            #[inline]
            fn div(self, rhs: &K) -> Self::Output {
                Self(self.0 / rhs)
            }
        }

        impl<K, V> std::ops::DivAssign<&K> for $generic_rate<V>
        where
            V: qrs_math::num::FloatBased
                + qrs_math::num::Vector<V::BaseFloat>
                + for<'a> std::ops::DivAssign<&'a K>,
        {
            #[inline]
            fn div_assign(&mut self, rhs: &K) {
                self.0 /= rhs;
            }
        }

        impl<V: qrs_math::num::Real> std::ops::Div<Duration> for $generic_rate<V> {
            type Output = Velocity<Self>;

            #[inline]
            fn div(self, rhs: Duration) -> Self::Output {
                Velocity::new(self, rhs)
            }
        }
    };
}

pub(super) use define_vector_behavior;
