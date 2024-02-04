mod cmp;
mod elementary_fns;
mod relpos;
mod scalar;
mod vector;

pub use cmp::TotalCmpForSort;
pub use elementary_fns::{Exp, Log};
pub use relpos::RelPos;
pub use scalar::{Arithmetic, FloatBased, Real, Scalar};
pub use vector::Vector;
