use crate::{Error, QueryChanged, QueryState};
use futures::Future;
use pin_project_lite::pin_project;
use std::{
    marker::PhantomData,
    pin::Pin,
    rc::Rc,
    task::{Context, Poll},
};

pin_project! {
    pub struct QueryFuture<T, Fut> {
        #[pin]
        fut: Fut,
        is_init: bool,
        on_change:  Option<Rc<dyn Fn(QueryChanged)>>,
        _marker: PhantomData<T>
    }
}

impl<T, Fut> QueryFuture<T, Fut> {
    pub fn new(fut: Fut, on_change: Option<Rc<dyn Fn(QueryChanged)>>) -> Self {
        QueryFuture {
            fut,
            is_init: false,
            on_change,
            _marker: PhantomData,
        }
    }
}

impl<T, Fut> Future for QueryFuture<T, Fut>
where
    Fut: Future<Output = Result<T, Error>>,
    T: 'static,
{
    type Output = Result<Rc<T>, Error>;

    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        let mut this = self.project();
        let ret = this.fut.as_mut().poll(cx);

        if !*this.is_init {
            *this.is_init = true;
            if let Some(callback) = this.on_change.as_ref() {
                callback(QueryChanged {
                    value: None,
                    state: QueryState::Loading,
                    is_fetching: true,
                })
            }
        }

        match ret {
            Poll::Ready(res) => {
                let res = res.map(Rc::new);

                if let Some(callback) = this.on_change {
                    match res.clone() {
                        Ok(value) => callback(QueryChanged {
                            value: Some(value),
                            state: QueryState::Ready,
                            is_fetching: false,
                        }),
                        Err(err) => callback(QueryChanged {
                            value: None,
                            state: QueryState::Failed(err),
                            is_fetching: false,
                        }),
                    }
                }

                Poll::Ready(res)
            }
            Poll::Pending => Poll::Pending,
        }
    }
}
