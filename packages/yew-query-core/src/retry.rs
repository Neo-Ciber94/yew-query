use std::{time::Duration, rc::Rc};

pub type Retry = Box<dyn Iterator<Item = Duration>>;

#[derive(Clone)]
pub struct Retryer(Rc<dyn Fn() -> Retry>);

impl Retryer {
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

    pub fn get(&self) -> Retry {
        (self.0)()
    }
}
