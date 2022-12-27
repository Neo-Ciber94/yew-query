use futures::{future::Shared, Future, FutureExt};
use pin_project_lite::pin_project;
use std::{cell::RefCell, rc::Rc};

pin_project! {
    /// A future that caches the resolved value.
    pub struct Cached<Fut>
    where
        Fut: Future,
    {
        last_value: Rc<RefCell<Option<Fut::Output>>>,

        #[pin]
        future_or_output: Shared<Fut>,
    }
}

impl<Fut> Cached<Fut>
where
    Fut: Future,
    Fut::Output: Clone,
{
    /// Constructs a new `Cached` future that caches the result of the given future.
    pub fn new(fut: Fut) -> Self {
        Self::with_initial(fut, None)
    }

    /// Constructs a new `Cached` future that caches the result of the given future 
    /// using the given value as the initial cache value.
    pub fn with_initial(fut: Fut, initial_value: Option<Fut::Output>) -> Self {
        Cached {
            future_or_output: fut.shared(),
            last_value: Rc::new(RefCell::new(initial_value)),
        }
    }

    /// Returns the last emitted value if any.
    pub fn last_value(&self) -> Option<Fut::Output> {
        match self.last_value.borrow().as_ref() {
            Some(x) => Some(x.clone()),
            None => None,
        }
    }

    /// Returns `true` if the future had resolved.
    pub fn is_resolved(&self) -> bool {
        self.future_or_output.peek().is_some()
    }
}

impl<Fut> Clone for Cached<Fut>
where
    Fut: Future,
{
    fn clone(&self) -> Self {
        Self {
            last_value: self.last_value.clone(),
            future_or_output: self.future_or_output.clone(),
        }
    }
}

impl<Fut> Future for Cached<Fut>
where
    Fut: Future,
    Fut::Output: Clone,
{
    type Output = Fut::Output;

    fn poll(
        self: std::pin::Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<Self::Output> {
        let this = self.project();
        match this.future_or_output.poll(cx) {
            std::task::Poll::Ready(x) => {
                *this.last_value.borrow_mut() = Some(x.clone());
                std::task::Poll::Ready(x)
            }
            std::task::Poll::Pending => std::task::Poll::Pending,
        }
    }
}

pub trait CacheFutureExt: Future {
    /// Returns a future that caches the last resolved value.
    fn cached(self) -> Cached<Self>
    where
        Self: Sized,
        Self::Output: Clone,
    {
        Cached::new(self)
    }

    /// Returns a future that caches the last resolved value using the given initial value.
    fn cached_with_initial(self, initial_value: Option<Self::Output>) -> Cached<Self>
    where
        Self: Sized,
        Self::Output: Clone,
    {
        Cached::with_initial(self, initial_value)
    }
}

impl<F> CacheFutureExt for F where F: Future {}
