// Copyright 2021 Vladislav Melnik
// SPDX-License-Identifier: MIT

use std::{
    ops::{Generator, GeneratorState},
    pin::Pin,
    cell::RefCell,
    fmt,
};
use super::algebra::{Effect, Context};

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

pub struct Computation<E, G>
where
    E: Effect,
    G: Unpin + Generator<(), Return = (), Yield = E::Input>,
{
    generator: G,
    context: Context<E>,
}

pub trait IntoComputation<E, G>
where
    E: Effect,
    G: Unpin + Generator<(), Return = (), Yield = E::Input>,
{
    fn into_computation(self) -> Computation<E, G>;
}

impl<F, E, G> IntoComputation<E, G> for F
where
    F: FnOnce(Context<E>) -> G,
    E: Effect,
    G: Unpin + Generator<(), Return = (), Yield = E::Input>,
{
    fn into_computation(self) -> Computation<E, G> {
        let context = Context::default();
        Computation {
            generator: self(context.clone()),
            context,
        }
    }
}

impl<E, G> Computation<E, G>
where
    E: Effect,
    G: Unpin + Generator<(), Return = (), Yield = E::Input>,
    G::Yield: fmt::Debug,
{
    pub fn run(self) {
        let Computation { mut generator, .. } = self;

        loop {
            match Pin::new(&mut generator).resume(()) {
                GeneratorState::Complete(()) => return,
                GeneratorState::Yielded(effects) => {
                    panic!("unhandled effects: {:?}", effects);
                },
            }
        }
    }

    pub fn add_handler<H>(
        self,
        handler: H,
    ) -> Computation<E, impl Unpin + Generator<(), Return = (), Yield = E::Input>>
    where
        H: Handler<E>,
    {
        let Computation {
            context,
            mut generator,
        } = self;
        let handler = RefCell::new(handler);
        Computation {
            context: context.clone(),
            generator: move || loop {
                match Pin::new(&mut generator).resume(()) {
                    GeneratorState::Complete(()) => return,
                    GeneratorState::Yielded(effects) => {
                        let mut h = handler.borrow_mut();
                        match h.handle(effects) {
                            Ok(handled) => *context.0.borrow_mut() = Some(handled),
                            Err(unhandled) => {
                                drop(h);
                                yield unhandled;
                            },
                        }
                    },
                }
            },
        }
    }
}
