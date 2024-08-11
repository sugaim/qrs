mod container;
mod error;
mod impls;

pub use container::{NonEmpty, RequireMinSize, SizeEnsured};
pub use error::Error;
pub use impls::{RefIntoIter, SizedContainer, SplitFirst};
