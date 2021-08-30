// Copyright 2021 Vladislav Melnik
// SPDX-License-Identifier: MIT

#![forbid(unsafe_code)]
#![feature(generators, generator_trait, never_type)]

#[cfg(feature = "aeiou-macros")]
pub use aeiou_macros::*;

mod computation;
pub use self::computation::{Handler, Effect, Select};

mod context;
pub use self::context::Context;

mod block;
pub use self::block::{Block, IntoBlock};

pub mod new;

#[macro_export]
macro_rules! perform {
    ($e:expr, $ctx:expr) => {{
        yield $e;
        aeiou::Select::take($ctx).unwrap()
    }};
    ($e:expr) => {{
        yield $e;
    }};
}
