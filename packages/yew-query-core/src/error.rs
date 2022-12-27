use crate::key::QueryKey;
use std::fmt::Display;
use std::sync::Arc;

#[derive(Clone)]
pub struct Error(Arc<dyn StdError + Send + Sync + 'static>);

impl Error {
    pub fn new<E>(error: E) -> Self
    where
        E: StdError + Send + Sync + 'static,
    {
        Error(Arc::new(error))
    }
}

impl std::error::Error for Error {}

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

pub trait StdError: std::fmt::Debug + std::fmt::Display {
    fn source(&self) -> Option<&(dyn StdError + 'static)> {
        None
    }
}

impl<E> StdError for E where E: std::fmt::Debug + std::fmt::Display {}

impl From<Error> for Box<dyn StdError + Send + Sync + 'static> {
    #[cold]
    fn from(error: Error) -> Self {
        Box::new(error)
    }
}

impl From<Error> for Box<dyn StdError + Send + 'static> {
    fn from(error: Error) -> Self {
        Box::<dyn StdError + Send + Sync>::from(error)
    }
}

impl From<Error> for Box<dyn StdError + 'static> {
    fn from(error: Error) -> Self {
        Box::<dyn StdError + Send + Sync>::from(error)
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
    TypeMismatch(TypeMismatchError),
    KeyNotFound(KeyNotFoundError),
    NoCacheValue,
}

impl QueryError {
    pub(crate) fn type_mismatch<T: 'static>() -> Self {
        let ty = std::any::type_name::<T>();
        QueryError::TypeMismatch(TypeMismatchError(ty))
    }

    pub(crate) fn key_not_found(key: &QueryKey) -> Self {
        let k = key.to_string();
        QueryError::KeyNotFound(KeyNotFoundError(k))
    }
}

impl std::error::Error for QueryError {}

impl Display for QueryError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        use QueryError::*;

        match self {
            TypeMismatch(TypeMismatchError(s)) => write!(f, "invalid type `{s}`"),
            KeyNotFound(KeyNotFoundError(k)) => write!(f, "key not found `{k}`"),
            NoCacheValue => write!(f, "no value in cache"),
        }
    }
}
