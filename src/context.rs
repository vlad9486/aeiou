// Copyright 2021 Vladislav Melnik
// SPDX-License-Identifier: MIT

use std::{rc::Rc, cell::RefCell};

pub struct Context<T>(Rc<RefCell<Option<T>>>);

impl<T> Context<T> {
    pub fn empty() -> Self {
        Context(Rc::new(RefCell::new(None)))
    }

    pub fn take(&self) -> Option<T> {
        self.0.borrow_mut().take()
    }

    pub fn put(&self, value: T) {
        *self.0.borrow_mut() = Some(value);
    }
}

impl<Output> Clone for Context<Output> {
    fn clone(&self) -> Self {
        Context(self.0.clone())
    }
}
