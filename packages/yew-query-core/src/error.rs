use crate::QueryKey;
use std::fmt::Display;
use std::sync::Arc;

use std::error::Error as StdError;

/// A cloneable error type.
#[derive(Clone)]
pub struct Error(Arc<dyn StdError + Send + Sync + 'static>);

impl Error {
    /// Constructs an error.
    pub fn new<E>(error: E) -> Self
    where
        E: StdError + Send + Sync + 'static,
    {
        Error(Arc::new(error))
    }
}

impl std::fmt::Debug for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:?}", self.0)
    }
}

impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl<E> From<E> for Error
where
    E: StdError + Send + Sync + 'static,
{
    #[cold]
    fn from(error: E) -> Self {
        Error::new(error)
    }
}

#[doc(hidden)]
#[derive(Debug)]
pub struct TypeMismatchError(&'static str);

#[doc(hidden)]
#[derive(Debug)]
pub struct KeyNotFoundError(String);

/// An error ocurred in a query.
#[derive(Debug)]
pub enum QueryError {
    /// If the value of the query cannot be converted to the given type.
    TypeMismatch(TypeMismatchError),

    /// If there is not query associated with a key.
    KeyNotFound(KeyNotFoundError),

    /// If the query exists but still fetching.
    NotReady,

    /// If the query exists but is stale.
    StaleValue,
}

impl QueryError {
    pub(crate) fn type_mismatch<T: 'static>() -> Self {
        let ty = std::any::type_name::<T>();
        QueryError::TypeMismatch(TypeMismatchError(ty))
    }

    pub(crate) fn key_not_found(key: &QueryKey) -> Self {
        QueryError::KeyNotFound(KeyNotFoundError(key.key().to_string()))
    }
}

impl std::error::Error for QueryError {}

impl Display for QueryError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        use QueryError::*;

        match self {
            TypeMismatch(TypeMismatchError(s)) => write!(f, "invalid type `{s}`"),
            KeyNotFound(KeyNotFoundError(k)) => write!(f, "key not found `{k}`"),
            NotReady => write!(f, "query had not resolved yet"),
            StaleValue => write!(f, "value is tale"),
        }
    }
}
