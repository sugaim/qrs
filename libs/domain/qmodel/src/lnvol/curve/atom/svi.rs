use anyhow::ensure;
use qfincore::{daycount::Act365f, Volatility};
use qmath::num::Real;

use crate::lnvol::{
    curve::{StrikeDer, VolCurve},
    LnCoord,
};

// -----------------------------------------------------------------------------
// Svi
// -----------------------------------------------------------------------------
/// SVI (Stochastic Volatility Inspired) model.
///
/// This uses the following formula:
/// variance = a + b * (rho * (y - m) + sqrt((y - m)^2 + sigma^2))
///
/// where y is log-moneyness(=ln(strike/forward)).
/// Note that this struct does not uses total variance as returned value.
/// If you are using total variance, please normalize before setting your
/// parameters to this struct.
#[derive(Clone, Debug, PartialEq, serde::Serialize, schemars::JsonSchema)]
pub struct Svi<V> {
    a: V,
    b: V,
    rho: V,
    m: V,
    sigma: V,
}

//
// serde
//
impl<'de, V: Real + serde::Deserialize<'de>> serde::Deserialize<'de> for Svi<V> {
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        #[derive(serde::Deserialize)]
        struct Inner<V> {
            a: V,
            b: V,
            rho: V,
            m: V,
            sigma: V,
        }
        let inner = Inner::deserialize(deserializer)?;
        Self::new(inner.a, inner.b, inner.rho, inner.m, inner.sigma)
            .map_err(serde::de::Error::custom)
    }
}

//
// ctor
//
impl<V: Real> Svi<V> {
    #[inline]
    pub fn new(a: V, b: V, rho: V, m: V, sigma: V) -> anyhow::Result<Self>
    where
        V: Real,
    {
        // rho must be in [-1, 1].
        // then, the non-linear part sqrt((y - m)^2 + sigma^2) is dominant
        // compare to the linear part rho * (y - m) and the curve is kept positive.
        ensure!(-V::one() <= rho, "rho must be in [-1, 1]");
        ensure!(rho <= V::one(), "rho must be in [-1, 1]");

        // b also controls convex downward or upward.
        // since the variance must be non-negative, b must be non-negative.
        ensure!(V::zero() <= b, "b must be non-negative");

        // when sigmpa is zero, the non-linear part, sqrt((y - m)^2 + sigma^2),
        // becomes abs(y - m) and the curve is not smooth.
        // the second positivity condition is just a conventional one.
        ensure!(!sigma.is_zero(), "sigma must be non-zero for smoothness");
        ensure!(V::zero() < sigma, "sigma must be positive.");

        // min is at y - m = - rho * sigma / sqrt(1 - rho^2).
        let min = a.clone() + &(b.clone() * &rho * &(V::one() - &(rho.clone() * &rho)).sqrt());

        // the second positivity condition just requires the variance to be positive.
        // althought zero seems allowed, this is not actually allowed because
        // derivative at the minimum diverges and non-smooth when the minimum is zero.
        ensure!(
            !min.is_zero(),
            "detected a + b * rho * sigma / sqrt(1 - rho^2) = 0 and this leads to non-smooth curve."
        );
        ensure!(
            V::zero() < min,
            "detected a + b * rho * sigma / sqrt(1 - rho^2) < 0 and this leads to negative variance."
        );

        Ok(Self {
            a,
            b,
            rho,
            m,
            sigma,
        })
    }
}

//
// methods
//
impl<V: Real> Svi<V> {
    #[inline]
    pub fn a(&self) -> &V {
        &self.a
    }

    #[inline]
    pub fn b(&self) -> &V {
        &self.b
    }

    #[inline]
    pub fn rho(&self) -> &V {
        &self.rho
    }

    #[inline]
    pub fn m(&self) -> &V {
        &self.m
    }

    #[inline]
    pub fn sigma(&self) -> &V {
        &self.sigma
    }
}

impl<V: Real> VolCurve for Svi<V> {
    type Value = V;

    #[inline]
    fn bsvol(
        &self,
        coord: &LnCoord<Self::Value>,
    ) -> anyhow::Result<Volatility<Act365f, Self::Value>> {
        let y = coord.clone().0 - &self.m;
        let non_lin = (y.clone().powi(2) + &self.sigma.clone().powi(2)).sqrt();
        let var = self.a.clone() + &(self.b.clone() * (y * &self.rho + &non_lin));
        Ok(Volatility {
            day_count: qfincore::daycount::Act365f,
            value: var.sqrt(),
        })
    }

    #[inline]
    fn bsvol_der(&self, coord: &LnCoord<V>) -> anyhow::Result<crate::lnvol::curve::StrikeDer<V>> {
        let y = coord.clone().0 - &self.m;
        let non_lin = (y.clone().powi(2) + &self.sigma.clone().powi(2)).sqrt(); // sigma != 0 by ctor. so non_lin != 0
        let var = self.a.clone() + &(self.b.clone() * (y.clone() * &self.rho + &non_lin));
        let vol = var.sqrt(); // vol != 0 by ctor.

        // dv^2/dy = 2 * v * dv/dy
        //  => dv/dy = (dv^2/dy) / (2 * v)
        let dv2dy = (self.rho.clone() + &(y / &non_lin)) * &self.b;
        let dvdy = V::nearest_value_of_f64(0.5) * &dv2dy / &vol;

        // d2v^2/dy^2 = 2 * [(dv/dy)^2 + v * d2v/dy^2]
        //  => d2v/dy^2 = (d2v^2/dy^2 - 0.5 * (dv/dy)^2) / v
        let d2v2dy2 = self.b.clone() * &self.sigma * &self.sigma / &non_lin.powi(3);
        let d2vdy2 = (d2v2dy2 - &(V::nearest_value_of_f64(0.5) * &dvdy.clone().powi(2))) / &vol;

        Ok(StrikeDer {
            vol: Volatility {
                day_count: Act365f,
                value: vol,
            },
            dvdy: Volatility {
                day_count: Act365f,
                value: dvdy,
            },
            d2vdy2: Volatility {
                day_count: Act365f,
                value: d2vdy2,
            },
        })
    }
}
