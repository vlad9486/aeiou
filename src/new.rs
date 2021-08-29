// Copyright 2021 Vladislav Melnik
// SPDX-License-Identifier: MIT

use std::{
    rc::Rc,
    cell::RefCell,
    pin::Pin,
    ops::{Generator, GeneratorState},
};

use either::Either;

pub trait EffectOutput {
    type Input: EffectInput<Output = Self>;
}

pub trait EffectInput {
    type Output: EffectOutput<Input = Self>;
}

impl EffectOutput for ! {
    type Input = !;
}

impl EffectInput for ! {
    type Output = !;
}

impl<L, R> EffectOutput for Either<L, R>
where
    L: EffectOutput,
    R: EffectOutput,
{
    type Input = Either<L::Input, R::Input>;
}

impl<L, R> EffectInput for Either<L, R>
where
    L: EffectInput,
    R: EffectInput,
{
    type Output = Either<L::Output, R::Output>;
}

pub struct Context<Output>(Rc<RefCell<Option<Output>>>)
where
    Output: EffectOutput;

impl<Output> Context<Output>
where
    Output: EffectOutput,
{
    pub fn empty() -> Self {
        Context(Rc::new(RefCell::new(None)))
    }
}

impl<Output> Clone for Context<Output>
where
    Output: EffectOutput,
{
    fn clone(&self) -> Self {
        Context(self.0.clone())
    }
}

pub trait EffectHandler {
    type Input: EffectInput;

    fn handle(&mut self, input: Self::Input) -> <Self::Input as EffectInput>::Output;
}

pub type Unhandled<Handler, Input> = <Input as Select<<Handler as EffectHandler>::Input>>::Rest;

pub trait Select<Part>
where
    Self: EffectInput,
    Part: EffectInput,
{
    type Rest: EffectInput;

    fn take_input(self) -> Either<Part, Self::Rest>;
    fn take_output(v: Self::Output) -> Either<Part::Output, <Self::Rest as EffectInput>::Output>;

    fn wrap_output(v: Part::Output) -> Self::Output;
}

pub trait Selected<Total>
where
    Self: EffectInput,
    Total: EffectInput,
{
    type Rest: EffectInput;

    fn wrap_output(v: Self::Output) -> Total::Output;
}

impl<L, R> Select<L> for Either<L, R>
where
    L: EffectInput,
    R: EffectInput,
{
    type Rest = R;

    fn take_input(self) -> Either<L, Self::Rest> {
        self
    }

    fn take_output(v: Self::Output) -> Either<L::Output, R::Output> {
        v
    }

    fn wrap_output(v: L::Output) -> Self::Output {
        Either::Left(v)
    }
}

impl<Input> Select<Input> for Input
where
    Input: EffectInput,
{
    type Rest = !;

    fn take_input(self) -> Either<Input, Self::Rest> {
        Either::Left(self)
    }

    fn take_output(v: Self::Output) -> Either<Input::Output, <Self::Rest as EffectInput>::Output> {
        Either::Left(v)
    }

    fn wrap_output(v: Input::Output) -> Self::Output {
        v
    }
}

impl<L, R> Selected<Either<L, R>> for R
where
    L: EffectInput,
    R: EffectInput,
{
    type Rest = L;

    fn wrap_output(v: Self::Output) -> <Either<L, R> as EffectInput>::Output {
        Either::Right(v)
    }
}

impl<Input> Selected<Input> for Input
where
    Input: EffectInput,
{
    type Rest = !;

    fn wrap_output(v: Self::Output) -> Input::Output {
        v
    }
}

pub struct Block<Output, G>
where
    Output: EffectOutput,
    G: Unpin + Generator<(), Return = ()>,
    G::Yield: EffectInput,  //Selected<Output::Input>
{
    context: Context<Output>,
    generator: G,
}

pub trait IntoBlock<Output, G>
where
    Output: EffectOutput,
    G: Unpin + Generator<(), Return = ()>,
    G::Yield: EffectInput + Selected<Output::Input>,
{
    fn into_block(self) -> Block<Output, G>;
}

impl<F, Output, G> IntoBlock<Output, G> for F
where
    F: FnOnce(Context<Output>) -> G,
    Output: EffectOutput,
    G: Unpin + Generator<(), Return = ()>,
    G::Yield: EffectInput + Selected<Output::Input>,
{
    fn into_block(self) -> Block<Output, G> {
        let context = Context::empty();
        Block {
            context: context.clone(),
            generator: self(context),
        }
    }
}

impl<Output, G> Block<Output, G>
where
    Output: EffectOutput,
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

impl<Output, G> Block<Output, G>
where
    Output: EffectOutput,
    G: Unpin + Generator<(), Return = ()>,
    G::Yield: EffectInput + Selected<Output::Input>,
{
    pub fn add_handler<Handler>(
        self,
        handler: Handler,
    ) -> Block<Output, impl Generator<(), Return = (), Yield = Unhandled<Handler, G::Yield>>>
    where
        Handler: EffectHandler,
        G::Yield: Select<Handler::Input>,
    {
        let Block { context, mut generator } = self;
        let handler = RefCell::new(handler);
        Block {
            context: context.clone(),
            generator: move || loop {
                match Pin::new(&mut generator).resume(()) {
                    GeneratorState::Complete(()) => return,
                    GeneratorState::Yielded(p) => match p.take_input() {
                        Either::Left(input) => {
                            let handled = handler.borrow_mut().handle(input);
                            let wrapped = <G::Yield as Select<Handler::Input>>::wrap_output(handled);
                            let wrapped = <G::Yield as Selected<Output::Input>>::wrap_output(wrapped);
                            *context.0.borrow_mut() = Some(wrapped);
                        },
                        Either::Right(rest) => yield rest,
                    },
                }
            },
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn simple() {
        pub struct A(u16);

        pub struct AOut(u8);

        pub struct AHandler;

        impl EffectOutput for AOut {
            type Input = A;
        }

        impl EffectInput for A {
            type Output = AOut;
        }

        impl EffectHandler for AHandler {
            type Input = A;

            fn handle(&mut self, input: Self::Input) -> <Self::Input as EffectInput>::Output {
                AOut((input.0 >> 8) as u8)
            }
        }

        pub struct B(u64);

        pub struct BOut(u32);

        pub struct BHandler;

        impl EffectOutput for BOut {
            type Input = B;
        }

        impl EffectInput for B {
            type Output = BOut;
        }

        impl EffectHandler for BHandler {
            type Input = B;

            fn handle(&mut self, input: Self::Input) -> <Self::Input as EffectInput>::Output {
                BOut((input.0 >> 32) as u32)
            }
        }

        let g = move |_context: Context<Either<AOut, BOut>>| move || {
            yield Either::Left(A(42));
            yield Either::Right(B(43));
        };
        g
            .into_block()
            .add_handler(AHandler)
            .add_handler(BHandler)
            .run();
    }
}
