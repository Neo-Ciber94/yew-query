use yew::{hook, use_effect_with_deps, use_mut_ref};

#[hook]
pub fn use_is_first_render() -> bool {
    let first_render_ref = use_mut_ref(|| true);
    let first_render = *first_render_ref.borrow();

    use_effect_with_deps(
        move |_| {
            *first_render_ref.borrow_mut() = false;
        },
        (),
    );

    first_render
}
