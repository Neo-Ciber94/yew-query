use std::fmt::Display;

use yew::virtual_dom::Key;

#[doc(hidden)]
#[derive(Debug)]
pub struct TypeMismatchError(&'static str);

#[doc(hidden)]
#[derive(Debug)]
pub struct KeyNotFoundError(String);

#[derive(Debug)]
pub enum QueryClientError {
    TypeMismatch(TypeMismatchError),
    KeyNotFound(KeyNotFoundError),
}

impl QueryClientError {
    pub(crate) fn type_mismatch<T: 'static>() -> Self {
        let ty = std::any::type_name::<T>();
        QueryClientError::TypeMismatch(TypeMismatchError(ty))
    }

    pub(crate) fn key_not_found(key: &Key) -> Self {
        let k = key.to_string();
        QueryClientError::KeyNotFound(KeyNotFoundError(k))
    }
}

impl std::error::Error for QueryClientError {}

impl Display for QueryClientError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        use QueryClientError::*;

        match self {
            TypeMismatch(TypeMismatchError(s)) => write!(f, "invalid type `{s}`"),
            KeyNotFound(KeyNotFoundError(k)) => write!(f, "key not found `{k}`"),
        }
    }
}
