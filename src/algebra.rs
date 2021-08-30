// Copyright 2021 Vladislav Melnik
// SPDX-License-Identifier: MIT

use super::context::Context;

pub trait Effect {
    type Input;
}

pub trait Select<Part>
where
    Self: Sized + Effect,
{
    fn take(output: &Context<Self>) -> Option<Part>;
}
