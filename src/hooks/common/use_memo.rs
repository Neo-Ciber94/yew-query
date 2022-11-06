use std::{borrow::Borrow, rc::Rc};
use yew::use_mut_ref;

// https://api.yew.rs/next/src/yew/functional/hooks/use_memo.rs.html

fn use_memo_base<T, F, D, K>(f: F, deps: D) -> Rc<T>
where
    T: 'static,
    F: FnOnce(D) -> (T, K),
    K: 'static + Borrow<D>,
    D: PartialEq,
{
    struct MemoState<T, K> {
        memo_key: K,
        result: Rc<T>,
    }
    let state = use_mut_ref(|| -> Option<MemoState<T, K>> { None });

    let mut state = state.borrow_mut();
    match &*state {
        Some(existing) if existing.memo_key.borrow() != &deps => {
            // Drop old state if it's outdated
            *state = None;
        }
        _ => {}
    };
    let state = state.get_or_insert_with(|| {
        let (result, memo_key) = f(deps);
        let result = Rc::new(result);
        MemoState { result, memo_key }
    });
    state.result.clone()
}

pub fn use_memo<T, F, D>(f: F, deps: D) -> Rc<T>
where
    T: 'static,
    F: FnOnce(&D) -> T,
    D: 'static + PartialEq,
{
    use_memo_base(|d| (f(&d), d), deps)
}
