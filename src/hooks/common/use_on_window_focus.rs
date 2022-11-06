use wasm_bindgen::{prelude::Closure, JsCast};
use web_sys::{window, Event};
use yew::use_effect;

use super::use_is_first_render::use_is_first_render;

pub fn use_on_window_focus<F>(callback: F)
where
    F: Fn() + 'static,
{
    let first_render = use_is_first_render();

    use_effect(move || {
        let window = window().unwrap();
        let cleanup = || ();

        if first_render {
            return cleanup;
        }

        let cb = Closure::wrap(Box::new(move |_: Event| callback()) as Box<dyn FnMut(_)>);

        window
            .add_event_listener_with_callback("focus", &cb.as_ref().unchecked_ref())
            .unwrap();

        cb.forget();
        cleanup
    });
}
