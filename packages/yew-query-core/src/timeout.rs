use std::rc::Rc;
use wasm_bindgen::prelude::Closure;
use wasm_bindgen::JsCast;
use web_sys::window;

// TODO: Remove?

pub struct Timeout(i32);

impl Timeout {
    pub fn new<F>(millis: u32, f: F) -> Self
    where
        F: Fn() + 'static,
    {
        let window = window().expect("expected `window`");
        let handler = Rc::new(Closure::wrap(Box::new(move || f()) as Box<dyn FnMut()>));
        let millis = i32::try_from(millis).expect("millis is too large");
        
        let id = window
            .set_timeout_with_callback_and_timeout_and_arguments_0(
                &(&*handler).as_ref().unchecked_ref(),
                millis,
            )
            .expect("failed to set timeout");

        Timeout(id)
    }

    pub fn clear(self) {
        let window = window().expect("expected `window`");
        window.clear_timeout_with_handle(self.0);
    }
}
