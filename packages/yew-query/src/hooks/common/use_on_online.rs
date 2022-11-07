use wasm_bindgen::{prelude::Closure, JsCast};
use web_sys::{window, Event};
use yew::use_effect_with_deps;

pub fn use_on_online<F>(callback: F)
where
    F: Fn() + 'static,
{
    use_effect_with_deps(
        move |_| {
            let window = window().unwrap();

            let on_online =
                { Closure::wrap(Box::new(move |_: Event| callback()) as Box<dyn FnMut(_)>) };

            window
                .add_event_listener_with_callback("online", &on_online.as_ref().unchecked_ref())
                .unwrap();

            on_online.forget();
            || {}
        },
        (),
    );
}
