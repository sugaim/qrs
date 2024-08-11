// -----------------------------------------------------------------------------
// Error
// -----------------------------------------------------------------------------
#[derive(Debug, thiserror::Error, PartialEq, Eq)]
pub enum Error<K> {
    #[error("Variable '{0:?}' is already instantiated")]
    VarAlreadyExists(K),
    #[error("Different graphs are used for an operation '{0}'")]
    DifferentGraphs(&'static str),
}
