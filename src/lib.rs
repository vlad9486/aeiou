// Copyright 2021 Vladislav Melnik
// SPDX-License-Identifier: MIT

// TODO: lazy handlers

#![forbid(unsafe_code)]
#![feature(generators, generator_trait)]

#[cfg(feature = "aeiou-macros")]
pub use aeiou_macros::*;

mod algebra;
pub use self::algebra::{Effect, Composable, Context};

mod computation;
pub use self::computation::{Computation, IntoComputation, Handler};

#[macro_export]
macro_rules! perform {
    ($e:expr, $ctx:expr) => {{
        yield $e;
        $ctx.take().unwrap()
    }};
    ($e:expr) => {{
        yield $e;
    }};
}
