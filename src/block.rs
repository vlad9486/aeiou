// Copyright 2021 Vladislav Melnik
// SPDX-License-Identifier: MIT

use std::{pin::Pin, ops::{Generator, GeneratorState}};
use super::context::Context;

pub struct Block<T, G>
where
    G: Unpin + Generator<(), Return = ()>,
{
    context: Context<T>,
    generator: G,
}

pub trait IntoBlock<T, G>
where
    G: Unpin + Generator<(), Return = ()>,
{
    fn into_block(self) -> Block<T, G>;
}

impl<F, T, G> IntoBlock<T, G> for F
where
    F: FnOnce(Context<T>) -> G,
    G: Unpin + Generator<(), Return = ()>,
{
    fn into_block(self) -> Block<T, G> {
        let context = Context::empty();
        Block {
            context: context.clone(),
            generator: self(context),
        }
    }
}

impl<T, G> Block<T, G>
where
    G: Unpin + Generator<(), Return = (), Yield = !>,
{
    pub fn run(self) {
        let Block { mut generator, .. } = self;
        match Pin::new(&mut generator).resume(()) {
            GeneratorState::Complete(()) => (),
            GeneratorState::Yielded(_) => unreachable!(),
        }
    }
}

impl<T, G> Block<T, G>
where
    G: Unpin + Generator<(), Return = ()>,
{
    // TODO: remove this
    pub(super) fn new(context: Context<T>, generator: G) -> Self {
        Block {
            context,
            generator,
        }
    }

    pub fn resume(&mut self) -> GeneratorState<G::Yield, G::Return> {
        Pin::new(&mut self.generator).resume(())
    }

    pub fn put(&self, value: T) {
        self.context.put(value);
    }

    pub fn context(&self) -> Context<T> {
        self.context.clone()
    }
}
