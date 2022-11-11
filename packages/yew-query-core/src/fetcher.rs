use super::Error;
use futures::Future;
use std::pin::Pin;

pub type TryBoxFuture<T, E = Error> = Pin<Box<dyn Future<Output = Result<T, E>>>>;

pub struct Fetcher<T>(Box<dyn Fn() -> TryBoxFuture<T>>);

impl<T> Fetcher<T> {
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

        Fetcher(f)
    }

    pub fn get(&self) -> TryBoxFuture<T> {
        (self.0)()
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
