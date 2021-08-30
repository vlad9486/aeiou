// Copyright 2021 Vladislav Melnik
// SPDX-License-Identifier: MIT

use std::{
    cell::RefCell,
    pin::Pin,
    ops::{Generator, GeneratorState},
    collections::BTreeMap,
};
use either::Either;
use super::context::Context;

pub trait TaskId {
    type Id: Eq + Ord;

    fn task_id(&self) -> Self::Id;
}

pub trait Request
where
    Self: Sized,
{
    type Task: TaskId;
    type Effect;

    fn is_task(self) -> Result<Self::Task, Self>;
    fn is_effect(self) -> Result<Self::Effect, Self>;
}

impl TaskId for ! {
    type Id = !;

    fn task_id(&self) -> Self::Id {
        loop {}
    }
}

impl Request for ! {
    type Task = !;
    type Effect = !;

    fn is_task(self) -> Result<Self::Task, Self> {
        loop {}
    }

    fn is_effect(self) -> Result<Self::Effect, Self> {
        loop {}
    }
}

pub struct Block<Output, G>
where
    G: Unpin + Generator<(), Return = ()>,
    G::Yield: Request,
{
    context: Context<Output>,
    generator: G,
}

pub trait IntoBlock<Output, G>
where
    G: Unpin + Generator<(), Return = ()>,
    G::Yield: Request,
{
    fn into_block(self) -> Block<Output, G>;
}

impl<F, Output, G> IntoBlock<Output, G> for F
where
    F: FnOnce(Context<Output>) -> G,
    G: Unpin + Generator<(), Return = ()>,
    G::Yield: Request,
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
    G: Unpin + Generator<(), Return = (), Yield = !>,
    G::Yield: Request,
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
    G: Unpin + Generator<(), Return = ()>,
    G::Yield: Request,
{
    pub fn spawn<F, T>(
        self,
        task_gen: F,
    ) -> Block<Output, impl Generator<(), Return = (), Yield = G::Yield>>
    where
        F: Fn(<G::Yield as Request>::Task) -> T,
        T: Unpin + Generator<(), Return = (), Yield = Either<G::Yield, Output>>,
    {
        let Block { context, generator } = self;
        Block {
            context: context.clone(),
            generator: move || {
                let mut generator = Some(generator);
                let mut tasks = BTreeMap::new();
                loop {
                    if let Some(g) = generator.as_mut() {
                        match Pin::new(g).resume(()) {
                            GeneratorState::Complete(()) => {
                                let _ = generator.take();
                            },
                            GeneratorState::Yielded(y) => match y.is_task() {
                                Ok(task) => {
                                    tasks.insert(task.task_id(), task_gen(task));
                                },
                                Err(y) => yield y,
                            },
                        }
                    }
                    let mut new_tasks = BTreeMap::new();
                    for (id, mut task) in tasks {
                        match Pin::new(&mut task).resume(()) {
                            GeneratorState::Complete(()) => (),
                            GeneratorState::Yielded(y) => {
                                new_tasks.insert(id, task);
                                match y {
                                    Either::Left(further) => yield further,
                                    Either::Right(output) => context.put(output),
                                }
                            },
                        }
                    }
                    tasks = new_tasks;

                    if generator.is_none() && tasks.is_empty() {
                        break;
                    }
                }
            },
        }
    }

    pub fn add_handler<Handler, NewYield>(
        self,
        handler: Handler,
    ) -> Block<Output, impl Generator<(), Return = (), Yield = NewYield>>
    where
        Handler: FnMut(<G::Yield as Request>::Effect) -> Result<Output, NewYield>,
        NewYield: Request,
    {
        let Block {
            context,
            mut generator,
        } = self;
        let handler = RefCell::new(handler);
        Block {
            context: context.clone(),
            generator: move || loop {
                match Pin::new(&mut generator).resume(()) {
                    GeneratorState::Complete(()) => break,
                    GeneratorState::Yielded(y) => {
                        if let Ok(effect) = y.is_effect() {
                            let mut h = handler.borrow_mut();
                            match h(effect) {
                                Ok(output) => context.put(output),
                                Err(y) => {
                                    drop(h);
                                    yield y;
                                },
                            }
                        }
                    },
                }
            },
        }
    }
}

#[cfg(test)]
mod tests {
    use std::{
        net::{SocketAddr, TcpListener, TcpStream},
        collections::BTreeMap,
        io::{Read, Write},
    };

    use either::Either;

    use super::{IntoBlock, Context, TaskId, Request};

    #[test]
    fn simple_tcp() {
        #[derive(Debug)]
        enum Req {
            ThrowEffect(Effect),
            Spawn(Task),
        }

        #[derive(Debug)]
        enum Effect {
            Listen(u16),
            Accept,
            Connect(SocketAddr),
            Read(SocketAddr, Vec<u8>, usize),
            Write(SocketAddr, Vec<u8>, usize),
        }

        enum Response {
            Listening,
            Accepted(SocketAddr),
            Connected(SocketAddr),
            DidRead(SocketAddr, Vec<u8>, usize),
            DidWrite(SocketAddr, Vec<u8>, usize),
        }

        #[derive(Debug)]
        pub struct Task(SocketAddr, bool);

        impl TaskId for Task {
            type Id = SocketAddr;

            fn task_id(&self) -> Self::Id {
                self.0.clone()
            }
        }

        impl Request for Req {
            type Task = Task;
            type Effect = Effect;

            fn is_task(self) -> Result<Self::Task, Self> {
                match self {
                    Req::Spawn(task) => Ok(task),
                    s => Err(s),
                }
            }

            fn is_effect(self) -> Result<Self::Effect, Self> {
                match self {
                    Req::ThrowEffect(effect) => Ok(effect),
                    s => Err(s),
                }
            }
        }

        let g = move |context: Context<Response>| {
            move || {
                yield Req::ThrowEffect(Effect::Listen(8224));
                yield Req::ThrowEffect(Effect::Connect(([127, 0, 0, 1], 8224).into()));
                loop {
                    match context.take() {
                        Some(Response::Accepted(addr)) => yield Req::Spawn(Task(addr, true)),
                        Some(Response::Connected(addr)) => {
                            yield Req::ThrowEffect(Effect::Accept);
                            yield Req::Spawn(Task(addr, false));
                        },
                        Some(Response::DidRead(addr, data, offset)) => {
                            println!(
                                "{} -> {:?}",
                                addr,
                                std::str::from_utf8(&data[..offset]).unwrap()
                            );
                        },
                        _ => (),
                    }
                }
            }
        };

        g.into_block()
            .spawn(move |Task(addr, incoming)| {
                move || {
                    println!("new: {}, incoming: {}", addr, incoming);
                    if incoming {
                        yield Either::Left(Req::ThrowEffect(Effect::Read(addr, vec![0; 0x10], 0)));
                    } else {
                        yield Either::Left(Req::ThrowEffect(Effect::Write(
                            addr,
                            b"hello, world\n".to_vec(),
                            0,
                        )));
                    }
                }
            })
            .add_handler({
                let mut listener = None::<TcpListener>;
                let mut streams = BTreeMap::new();
                move |effect: Effect| match effect {
                    Effect::Listen(port) => {
                        listener = Some(
                            TcpListener::bind::<SocketAddr>(([0, 0, 0, 0], port).into()).unwrap(),
                        );
                        Ok(Response::Listening)
                    },
                    Effect::Accept => {
                        let (s, addr) = listener.as_ref().unwrap().accept().unwrap();
                        streams.insert(addr, s);
                        Ok(Response::Accepted(addr))
                    },
                    Effect::Connect(addr) => {
                        streams.insert(addr, TcpStream::connect(addr).unwrap());
                        Ok(Response::Connected(addr))
                    },
                    Effect::Read(addr, mut buffer, mut offset) => {
                        if let Some(stream) = streams.get_mut(&addr) {
                            offset += stream.read(&mut buffer[offset..]).unwrap();
                        }
                        Ok(Response::DidRead(addr, buffer, offset))
                    },
                    Effect::Write(addr, buffer, mut offset) => {
                        if let Some(stream) = streams.get_mut(&addr) {
                            offset += stream.write(&buffer[offset..]).unwrap();
                        }
                        Ok(Response::DidWrite(addr, buffer, offset))
                    },
                }
            })
            .run();
    }
}
