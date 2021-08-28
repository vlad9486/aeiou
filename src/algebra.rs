// Copyright 2021 Vladislav Melnik
// SPDX-License-Identifier: MIT

use std::{rc::Rc, cell::RefCell};

pub struct Context<E>(pub Rc<RefCell<Option<E>>>)
where
    E: Effect;

impl<E> Default for Context<E>
where
    E: Effect,
{
    fn default() -> Self {
        Context(Rc::new(RefCell::new(None)))
    }
}

impl<E> Clone for Context<E>
where
    E: Effect,
{
    fn clone(&self) -> Self {
        Context(self.0.clone())
    }
}

impl<E> Context<E>
where
    E: Effect,
{
    pub fn take<Part>(&self) -> Option<Part>
    where
        E: Composable<Part>,
    {
        E::take(self)
    }
}

pub trait Effect {
    type Input;
}

pub trait Composable<Part>
where
    Self: Sized + Effect,
{
    fn take(output: &Context<Self>) -> Option<Part>;
}
