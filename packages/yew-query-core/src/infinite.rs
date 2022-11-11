use std::{cell::RefCell, rc::Rc};

pub struct InfiniteData<T> {
    pages: Rc<RefCell<Vec<T>>>,
}

impl<T> InfiniteData<T> {
    
}