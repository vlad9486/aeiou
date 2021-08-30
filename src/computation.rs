// Copyright 2021 Vladislav Melnik
// SPDX-License-Identifier: MIT

use std::{
    ops::{Generator, GeneratorState},
    cell::RefCell,
    fmt,
};
use super::{block::Block, context::Context};

pub trait Effect {
    type Input;
}

pub trait Select<Part>
where
    Self: Sized + Effect,
{
    fn take(output: &Context<Self>) -> Option<Part>;
}

pub trait Handler<E>
where
    E: Effect,
{
    fn handle(&mut self, effect: E::Input) -> Result<E, E::Input>;
}

impl<F, E> Handler<E> for F
where
    E: Effect,
    F: FnMut(E::Input) -> Result<E, E::Input>,
{
    fn handle(&mut self, effect: E::Input) -> Result<E, E::Input> {
        self(effect)
    }
}

impl<E, G> Block<E, G>
where
    E: Effect,
    G: Unpin + Generator<(), Return = (), Yield = E::Input>,
    G::Yield: fmt::Debug,
{
    pub fn assert_handled(self) -> Block<E, impl Unpin + Generator<(), Return = (), Yield = !>> {
        let context = self.context();
        let mut s = self;
        let generator = move || loop {
            match s.resume() {
                GeneratorState::Complete(()) => break,
                GeneratorState::Yielded(effects) => {
                    panic!("unhandled: {:?}", effects);
                    #[allow(unreachable_code)]
                    yield loop {}
                },
            }
        };
        Block::new(context, generator)
    }

    pub fn add_handler<H>(
        self,
        handler: H,
    ) -> Block<E, impl Unpin + Generator<(), Return = (), Yield = E::Input>>
    where
        H: Handler<E>,
    {
        let context = self.context();
        let handler = RefCell::new(handler);
        let mut s = self;
        let generator = move || loop {
            match s.resume() {
                GeneratorState::Complete(()) => return,
                GeneratorState::Yielded(effects) => {
                    let mut h = handler.borrow_mut();
                    match h.handle(effects) {
                        Ok(handled) => s.put(handled),
                        Err(unhandled) => {
                            drop(h);
                            yield unhandled;
                        },
                    }
                },
            }
        };
        Block::new(context, generator)
    }
}
