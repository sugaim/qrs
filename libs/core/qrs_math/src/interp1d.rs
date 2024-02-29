mod _knots;
mod chermite;
mod lerp;
mod pwconst;
mod traits;

pub use chermite::{CHermite1d, CHermite1dBuilder, CHermiteScheme, CatmullRomScheme};
pub use lerp::{Lerp1d, Lerp1dBuilder};
pub use pwconst::{PwConst1d, PwConst1dBuilder};
pub use traits::{DestructibleInterp1d, Interp1d, Interp1dBuilder};
