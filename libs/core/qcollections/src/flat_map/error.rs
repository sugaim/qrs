// -----------------------------------------------------------------------------
// Error
// -----------------------------------------------------------------------------
#[derive(Debug, Clone, thiserror::Error)]
pub enum Error {
    #[error("Unsortable keys found")]
    Unsortable,
    #[error("Unordered keys found")]
    Unordered,
    #[error("Duplicated keys found")]
    Duplicated,
    #[error("Size mismatch. keys: {}, values: {}", .keys, .values)]
    SizeMismatch { keys: usize, values: usize },
}
