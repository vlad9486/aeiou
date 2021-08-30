// Copyright 2021 Vladislav Melnik
// SPDX-License-Identifier: MIT

#![forbid(unsafe_code)]
#![feature(generators, generator_trait, never_type)]

#[cfg(feature = "aeiou-macros")]
pub use aeiou_macros::*;

mod algebra;
pub use self::algebra::{Effect, Select};

mod computation;
pub use self::computation::{Computation, IntoComputation, Handler};

pub mod new;

mod context;
pub use self::context::Context;

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
