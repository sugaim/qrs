// -----------------------------------------------------------------------------
// Error
// -----------------------------------------------------------------------------
#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord, thiserror::Error)]
#[error("Size is {}, which is less than required size {}", .actual, .required)]
pub struct Error {
    pub required: usize,
    pub actual: usize,
}
