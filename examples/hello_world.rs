// Copyright 2021 Vladislav Melnik
// SPDX-License-Identifier: MIT

#![feature(generators)]

use std::{
    collections::BTreeMap,
    net::{SocketAddr, TcpStream, TcpListener},
    io::{Read, Write},
    thread,
    time::Duration,
};
use aeiou::{Composable, Context, Effect, Handler, IntoComputation, perform};

#[derive(Debug)]
pub enum Effects {
    ListenTcp(u16),
    ConnectTcp(SocketAddr),
    ReadTcp(SocketAddr),
    WriteTcp(SocketAddr, String),
    Print(String),
}

#[derive(Effect, Composable)]
#[input(Effects)]
pub enum EffectsOutput {
    #[part(AcceptedTcp)]
    ListenedTcp(SocketAddr),
    ConnectedTcp(SocketAddr),
    #[part(ReadTcp)]
    ReadTcp(String),
    WrittenTcp,
    Printed,
}

pub struct AcceptedTcp(SocketAddr);

pub struct ReadTcp(String);

#[derive(Default)]
pub struct TcpHandler {
    listener: Option<TcpListener>,
    streams: BTreeMap<SocketAddr, TcpStream>,
}

impl Handler<EffectsOutput> for TcpHandler {
    fn handle(&mut self, effect: Effects) -> Result<EffectsOutput, Effects> {
        match effect {
            Effects::ListenTcp(port) => {
                let listener =
                    TcpListener::bind::<SocketAddr>(([0, 0, 0, 0], port).into()).unwrap();
                let (stream, addr) = listener.accept().unwrap();
                self.listener = Some(listener);
                self.streams.insert(addr, stream);
                Ok(EffectsOutput::ListenedTcp(addr))
            },
            Effects::ConnectTcp(addr) => {
                self.streams.insert(addr, TcpStream::connect(addr).unwrap());
                Ok(EffectsOutput::ConnectedTcp(addr))
            },
            Effects::ReadTcp(addr) => {
                let mut buffer = [0; 256];
                let read = self
                    .streams
                    .get_mut(&addr)
                    .unwrap()
                    .read(&mut buffer)
                    .unwrap();
                Ok(EffectsOutput::ReadTcp(
                    String::from_utf8(buffer[..read].to_vec()).unwrap(),
                ))
            },
            Effects::WriteTcp(addr, msg) => {
                self.streams
                    .get_mut(&addr)
                    .unwrap()
                    .write_all(msg.as_bytes())
                    .unwrap();
                Ok(EffectsOutput::WrittenTcp)
            },
            Effects::Print(_) => Err(effect),
        }
    }
}

fn main() {
    let server = |context: Context<EffectsOutput>| {
        move || {
            let AcceptedTcp(addr) = perform!(Effects::ListenTcp(8224), context);
            let ReadTcp(data) = perform!(Effects::ReadTcp(addr), context);
            perform!(Effects::Print(data));
        }
    };

    let client = |_: Context<EffectsOutput>| {
        move || {
            let addr = ([127, 0, 0, 1], 8224).into();
            perform!(Effects::ConnectTcp(addr));
            perform!(Effects::WriteTcp(addr, "hello world!\n".to_string()));
        }
    };

    let server_thread = thread::spawn(move || {
        server
            .into_computation()
            .add_handler(TcpHandler::default())
            .add_handler(|effect| match effect {
                Effects::Print(msg) => {
                    std::io::stdout().write(msg.as_bytes()).unwrap();
                    Ok(EffectsOutput::Printed)
                },
                _ => Err(effect),
            })
            .run()
    });
    thread::sleep(Duration::from_millis(10));
    client
        .into_computation()
        .add_handler(TcpHandler::default())
        .run();
    server_thread.join().unwrap();
}
