// Copyright 2021 Vladislav Melnik
// SPDX-License-Identifier: MIT

use std::{rc::Rc, cell::RefCell, pin::Pin, ops::{Generator, GeneratorState}};

use either::Either;

pub trait EffectOutput {
    type Input: EffectInput<Output = Self>;
}

pub trait EffectInput {
    type Output: EffectOutput<Input = Self>;
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

pub trait EffectHandler<Input>
where
    Input: EffectInput,
{
    fn handle(&mut self, input: Input) -> Input::Output;
}

pub trait Select<Part>
where
    Self: Sized + EffectInput,
    Part: EffectInput,
{
    type Rest: EffectInput;

    fn take_input(self) -> Either<Part, Self::Rest>;
    fn take_output(v: Self::Output) -> Either<Part::Output, <Self::Rest as EffectInput>::Output>;

    fn wrap_output(v: Part::Output) -> Self::Output;
}

pub trait CoSelect<Total>
where
    Self: Sized + EffectInput,
    Total: EffectInput,
{
    type Part: EffectInput;

    fn wrap_output(v: Self::Output) -> Total::Output;
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

impl EffectOutput for ! {
    type Input = !;
}

impl EffectInput for ! {
    type Output = !;
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

impl<L, R> CoSelect<Either<L, R>> for R
where
    L: EffectInput,
    R: EffectInput,
{
    type Part = L;

    fn wrap_output(v: Self::Output) -> <Either<L, R> as EffectInput>::Output {
        Either::Right(v)
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

impl<Input> CoSelect<Input> for Input
where
    Input: EffectInput,
{
    type Part = !;

    fn wrap_output(v: Self::Output) -> Input::Output {
        v
    }
}

pub fn handled<BaseOutput, G, PartInput, Handler>(
    handler: Handler,
    impure: G,
    context: Context<BaseOutput>,
) -> impl Generator<(), Return = (), Yield = <G::Yield as Select<PartInput>>::Rest>
where
    BaseOutput: EffectOutput,
    G: Unpin + Generator<(), Return = ()>,
    G::Yield: EffectInput + Select<PartInput> + CoSelect<BaseOutput::Input>,
    PartInput: EffectInput,
    Handler: EffectHandler<PartInput>,
{
    let mut impure = impure;
    let handler = RefCell::new(handler);
    move || {
        loop {
            match Pin::new(&mut impure).resume(()) {
                GeneratorState::Complete(()) => return,
                GeneratorState::Yielded(p) => {
                    match p.take_input() {
                        Either::Left(input) => {
                            let handled = handler.borrow_mut().handle(input);
                            let wrapped = <G::Yield as Select<PartInput>>::wrap_output(handled);
                            let wrapped = <G::Yield as CoSelect<BaseOutput::Input>>::wrap_output(wrapped);
                            *context.0.borrow_mut() = Some(wrapped);
                        },
                        Either::Right(rest) => yield rest,
                    }
                }
            }
        }
    }
}

pub fn run<BaseOutput, G>(pure: G)
where
    G: Unpin + Generator<(), Return = (), Yield = !>,
{
    let mut pure = pure;
    match Pin::new(&mut pure).resume(()) {
        GeneratorState::Complete(()) => (),
        GeneratorState::Yielded(_) => unreachable!(),
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

        pub struct AorBHandler;

        impl EffectOutput for AOut {
            type Input = A;
        }

        impl EffectInput for A {
            type Output = AOut;
        }

        impl EffectHandler<A> for AHandler {
            fn handle(&mut self, input: A) -> <A as EffectInput>::Output {
                AOut((input.0 >> 8) as u8)
            }
        }

        impl EffectHandler<Either<A, B>> for AorBHandler {
            fn handle(&mut self, input: Either<A, B>) -> <Either<A, B> as EffectInput>::Output {
                match input {
                    Either::Left(a) => Either::Left(AOut((a.0 >> 8) as u8)),
                    Either::Right(b) => Either::Right(BHandler.handle(b)),
                }
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

        impl EffectHandler<B> for BHandler {
            fn handle(&mut self, input: B) -> <B as EffectInput>::Output {
                BOut((input.0 >> 32) as u32)
            }
        }

        let context = Context::<Either<AOut, BOut>>::empty();
        let g = move || {
            yield Either::Left(A(42));
        };

        let a_handled = handled(AHandler, g, context.clone());
        let b_handled = handled(BHandler, a_handled, context.clone());
        run::<Either<AOut, BOut>, _>(b_handled)
    }
}
