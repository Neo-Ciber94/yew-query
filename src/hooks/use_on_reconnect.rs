use wasm_bindgen::{prelude::Closure, JsCast};
use web_sys::{window, Event};
use yew::{use_effect, use_state};

fn is_online() -> bool {
    let window = window().unwrap();
    let navigator = window.navigator();
    navigator.on_line()
}

pub fn use_on_reconnect<F>(callback: F)
where
    F: Fn() + 'static,
{
    let state = use_state(|| is_online());

    use_effect(move || {
        let window = window().unwrap();
        let prev_state = *state;

        let on_online = {
            let state = state.clone();
            Closure::wrap(Box::new(move |_: Event| {
                if prev_state == false {
                    callback()
                }

                state.set(true);
            }) as Box<dyn FnMut(_)>)
        };

        let on_offline = {
            Closure::wrap(Box::new(move |_: Event| {
                state.set(false);
            }) as Box<dyn FnMut(_)>)
        };

        window
            .add_event_listener_with_callback("online", &on_online.as_ref().unchecked_ref())
            .unwrap();

        window
            .add_event_listener_with_callback("offline", &on_offline.as_ref().unchecked_ref())
            .unwrap();

        on_online.forget();
        on_offline.forget();
        || {}
    });
}
