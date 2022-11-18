use super::Error;
use futures::{Future, TryFutureExt};
use std::pin::Pin;

pub type TryBoxFuture<T, E = Error> = Pin<Box<dyn Future<Output = Result<T, E>>>>;
pub struct BoxFetcher<T>(Box<dyn Fn() -> TryBoxFuture<T>>);

impl<T> BoxFetcher<T> {
    pub fn new<F, Fut, E>(fetcher: F) -> Self
    where
        F: Fn() -> Fut + 'static,
        Fut: Future<Output = Result<T, E>> + 'static,
        E: Into<Error> + 'static,
    {
        let f = Box::new(move || {
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

pub trait Fetch<T> {
    type Fut: Future<Output = Result<T, Error>>;
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
