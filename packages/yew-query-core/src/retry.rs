use std::{fmt::Debug, rc::Rc, time::Duration};

/// Represents an iterator over a duration.
pub type Retry = Box<dyn Iterator<Item = Duration>>;

/// Boxes a retry iterator.
#[derive(Clone)]
pub struct Retryer(Rc<dyn Fn() -> Retry>);

impl Retryer {
    /// Constructs a new `Retryer`.
    pub fn new<F, I>(f: F) -> Self
    where
        F: Fn() -> I + 'static,
        I: Iterator<Item = Duration> + 'static,
    {
        let f = Rc::new(move || {
            let retry = f();
            Box::new(retry) as Retry
        });

        Retryer(f)
    }

    /// Returns a `Retry` iterator.
    pub fn get(&self) -> Retry {
        (self.0)()
    }
}

impl Debug for Retryer {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Retryer")
    }
}
