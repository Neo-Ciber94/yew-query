use crate::listener::EventListener;
use yew::use_effect_with_deps;

pub fn use_on_online<F>(callback: F)
where
    F: Fn() + 'static,
{
    use_effect_with_deps(
        move |_| {
            let listener = EventListener::window("online", move |_| callback());

            move || {
                listener.unsubscribe();
            }
        },
        (),
    );
}
