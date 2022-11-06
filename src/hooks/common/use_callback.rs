use super::use_memo;
use std::{fmt, rc::Rc};

pub struct Callback<IN, OUT> {
    pub(crate) cb: Rc<dyn Fn(IN) -> OUT>,
}

impl<IN, OUT, F: Fn(IN) -> OUT + 'static> From<F> for Callback<IN, OUT> {
    fn from(func: F) -> Self {
        Callback { cb: Rc::new(func) }
    }
}

impl<IN, OUT> Clone for Callback<IN, OUT> {
    fn clone(&self) -> Self {
        Self {
            cb: self.cb.clone(),
        }
    }
}

#[allow(clippy::vtable_address_comparisons)]
impl<IN, OUT> PartialEq for Callback<IN, OUT> {
    fn eq(&self, other: &Callback<IN, OUT>) -> bool {
        let (Callback { cb }, Callback { cb: rhs_cb }) = (self, other);
        Rc::ptr_eq(cb, rhs_cb)
    }
}

impl<IN, OUT> fmt::Debug for Callback<IN, OUT> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Callback<_>")
    }
}

impl<IN, OUT> Callback<IN, OUT> {
    /// This method calls the callback's function.
    pub fn emit(&self, value: IN) -> OUT {
        (*self.cb)(value)
    }
}

pub fn use_callback<IN, OUT, F, D>(f: F, deps: D) -> Callback<IN, OUT>
where
    IN: 'static,
    OUT: 'static,
    F: Fn(IN, &D) -> OUT + 'static,
    D: PartialEq + 'static,
{
    let deps = Rc::new(deps);

    (*use_memo(
        move |deps| {
            let deps = deps.clone();
            let f = move |value: IN| f(value, deps.as_ref());
            Callback::from(f)
        },
        deps,
    ))
    .clone()
}
