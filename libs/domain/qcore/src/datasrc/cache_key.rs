use std::{fmt::Display, hash::Hash};

use serde::{Deserialize, Serialize};

// -----------------------------------------------------------------------------
// CacheKey
//

/// Required trait for a type to be used as a key for cache.
pub trait CacheKey: Send + Clone + Eq + Hash {}

impl<T> CacheKey for T where T: Send + Clone + Eq + Hash {}

// -----------------------------------------------------------------------------
// CacheKeyWorkaround
//

/// Customization point to define `Eq` and `Hash` for cache key without ordinary `Eq` and `Hash` implementation.
///
/// To use a type as a key for cache without ordinary `Eq` and `Hash` implementation,
/// such as `f32` and `f64`, this trait and `CacheKeyWrapper` are used.
///
/// Note that this trait may not obey logical equality.
/// For example, `+0.0` and `-0.0` are equal in ordinary mathematical sense,
/// but these are distinguished in equality provided by `CacheKeyWorkaround`.
///
/// So methods provided by this trait should be used only for cache or similar purpose.
///
pub trait CacheKeyWorkaround: Sized + Send + Clone {
    fn eq_for_cache(&self, other: &Self) -> bool;
    fn hash_for_cache<H>(&self, state: &mut H)
    where
        H: std::hash::Hasher;

    fn into_cache_key(self) -> CacheKeyWrapper<Self> {
        CacheKeyWrapper(self)
    }
}

impl CacheKeyWorkaround for f32 {
    #[inline]
    fn eq_for_cache(&self, other: &Self) -> bool {
        self.to_bits() == other.to_bits()
    }

    #[inline]
    fn hash_for_cache<H>(&self, state: &mut H)
    where
        H: std::hash::Hasher,
    {
        self.to_bits().hash(state);
    }
}

impl CacheKeyWorkaround for f64 {
    #[inline]
    fn eq_for_cache(&self, other: &Self) -> bool {
        self.to_bits() == other.to_bits()
    }

    #[inline]
    fn hash_for_cache<H>(&self, state: &mut H)
    where
        H: std::hash::Hasher,
    {
        self.to_bits().hash(state);
    }
}

impl<T: CacheKeyWorkaround> CacheKeyWorkaround for num::Complex<T> {
    #[inline]
    fn eq_for_cache(&self, other: &Self) -> bool {
        self.re.eq_for_cache(&other.re) && self.im.eq_for_cache(&other.im)
    }

    #[inline]
    fn hash_for_cache<H>(&self, state: &mut H)
    where
        H: std::hash::Hasher,
    {
        self.re.hash_for_cache(state);
        self.im.hash_for_cache(state);
    }
}

// -----------------------------------------------------------------------------
//
//

/// Customization point to define `Eq` and `Hash` for a type
/// which does not implement `Eq` and `Hash` itself.
///
/// For example, `f32` and `f64` do not implement `Eq` and `Hash`
/// but we sometimes want to use them as a key for cache.
/// To achieve this, `CacheKeyWorkaround` is used.
///
/// Note that this trait may not obey logical equality.
/// For example, `+0.0` and `-0.0` are equal in ordinary mathematical sense,
/// but these are distinguished in equality provided by `CacheKeyWorkaround`.
///
/// So methods provided by this trait should be used only for cache or similar purpose.
///
/// # Example
/// ```
/// use std::collections::HashMap;
/// use std::sync::Mutex;
/// use qcore::datasrc::CacheKeyWorkaround;
/// use qcore::datasrc::CacheKeyWrapper;
///
/// struct Calculator {
///     cache: Mutex<HashMap<CacheKeyWrapper<f64>, f64>>,
/// }
///
/// impl Calculator {
///     fn new() -> Self {
///         Self {
///            cache: Mutex::new(HashMap::new()),
///         }
///     }
///
///     // `Ok` for cache hit, `Err` for cache miss
///     fn calculate(&self, x: f64) -> Result<f64, f64> {
///         let mut cache = self.cache.lock().unwrap();
///         let key = x.into();
///         if let Some(&y) = cache.get(&key) {
///             return Ok(y);
///         }
///         let y = x * x + 4.2;
///         cache.insert(key, y);
///         Err(y)
///     }
/// }
///
/// let calc = Calculator::new();
/// assert_eq!(calc.calculate(3.0), Err(13.2));
/// assert_eq!(calc.calculate(3.0), Ok(13.2));
/// assert_eq!(calc.calculate(0.0), Err(4.2));
/// assert_eq!(calc.calculate(-0.0), Err(4.2)); // +0.0 and -0.0 are distinguished
/// ```
///

// -----------------------------------------------------------------------------
// CacheKeyWrapper
//
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
#[repr(transparent)]
#[serde(transparent)]
pub struct CacheKeyWrapper<T>(T);

impl<T: CacheKey> AsRef<T> for CacheKeyWrapper<T> {
    fn as_ref(&self) -> &T {
        &self.0
    }
}

//
// display, serde
//
impl<T: CacheKeyWorkaround + Display> Display for CacheKeyWrapper<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.0.fmt(f)
    }
}

//
// comparison
//
impl<T: CacheKeyWorkaround> PartialEq for CacheKeyWrapper<T> {
    fn eq(&self, other: &Self) -> bool {
        self.0.eq_for_cache(&other.0)
    }
}

impl<T: CacheKeyWorkaround> Eq for CacheKeyWrapper<T> {}

impl<T: CacheKeyWorkaround> Hash for CacheKeyWrapper<T> {
    fn hash<H>(&self, state: &mut H)
    where
        H: std::hash::Hasher,
    {
        self.0.hash_for_cache(state);
    }
}

//
// construction
//
impl<T: CacheKeyWorkaround> From<T> for CacheKeyWrapper<T> {
    fn from(t: T) -> Self {
        CacheKeyWrapper(t)
    }
}

impl<T: CacheKeyWorkaround> CacheKeyWrapper<T> {
    pub fn new(t: T) -> Self {
        CacheKeyWrapper(t)
    }
}

//
// methods
//
impl<T: CacheKeyWorkaround> CacheKeyWrapper<T> {
    pub fn into_inner(self) -> T {
        self.0
    }
}
