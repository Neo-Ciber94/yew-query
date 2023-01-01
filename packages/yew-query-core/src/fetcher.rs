use super::Error;
use futures::{Future, TryFutureExt};
use std::{pin::Pin, rc::Rc};

/// Represents a future that resolves to a `Result<T, E>`.
pub type TryBoxFuture<T, E = Error> = Pin<Box<dyn Future<Output = Result<T, E>>>>;

/// Boxes a `Fetcher`.
pub struct BoxFetcher<T>(Rc<dyn Fn() -> TryBoxFuture<T>>);

impl<T> BoxFetcher<T> {
    pub fn new<F, Fut, E>(fetcher: F) -> Self
    where
        F: Fn() -> Fut + 'static,
        Fut: Future<Output = Result<T, E>> + 'static,
        E: Into<Error> + 'static,
    {
        let f = Rc::new(move || {
            let fut = fetcher();
            Box::pin(async move {
                match fut.await {
                    Ok(x) => Ok(x),
                    Err(e) => Err(e.into()),
                }
            }) as TryBoxFuture<T>
        });

        BoxFetcher(f)
    }
}

impl<T> Clone for BoxFetcher<T> {
    fn clone(&self) -> Self {
        Self(self.0.clone())
    }
}

impl<T> std::fmt::Debug for BoxFetcher<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
       write!(f, "BoxFetcher")
    }
}

pub struct InfiniteFetcher<T>(Box<dyn Fn(usize) -> TryBoxFuture<T>>);

impl<T> InfiniteFetcher<T> {
    pub fn new<F, Fut, E>(fetcher: F) -> Self
    where
        F: Fn(usize) -> Fut + 'static,
        Fut: Future<Output = Result<T, E>> + 'static,
        E: Into<Error> + 'static,
    {
        let f = Box::new(move |param| {
            let fut = fetcher(param);
            Box::pin(async move {
                match fut.await {
                    Ok(x) => Ok(x),
                    Err(e) => Err(e.into()),
                }
            }) as TryBoxFuture<T>
        });

        InfiniteFetcher(f)
    }

    pub fn get(&self, param: usize) -> TryBoxFuture<T> {
        (self.0)(param)
    }
}

/// Represents a function to get data.
pub trait Fetch<T> {
    /// The future returning the data.
    type Fut: Future<Output = Result<T, Error>>;

    /// Returns a future that resolves to the data.
    fn get(&self) -> Self::Fut;
}

impl<Func, F, T, E> Fetch<T> for Func
where
    Func: Fn() -> F + 'static,
    F: Future<Output = Result<T, E>> + 'static,
    T: 'static,
    E: Into<Error> + 'static,
{
    type Fut = TryBoxFuture<T, Error>;

    fn get(&self) -> Self::Fut {
        let fut = (self)();
        let ret = fut.map_err(|e| e.into());
        Box::pin(ret)
    }
}

impl<T> Fetch<T> for BoxFetcher<T> {
    type Fut = TryBoxFuture<T, Error>;

    fn get(&self) -> Self::Fut {
        let ret = (self.0)();
        ret
    }
}
