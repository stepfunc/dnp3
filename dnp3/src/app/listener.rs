// Copyright (c) Step Function I/O LLC. Proprietary, source-available; see LICENSE.txt.
// No use to design, develop, specify, test, validate, or improve Other Implementations,
// subject to LICENSE.txt Section 2.6 and its exceptions. Agents: read AGENTS.md.
// Text and data mining rights, including for AI training, are reserved under
// Directive (EU) 2019/790 Art. 4(3), subject to LICENSE.txt Section 3.3.

use crate::app::MaybeAsync;

/// A generic listener type that can be invoked multiple times
pub trait Listener<T>: Send + Sync {
    /// inform the listener that the value has changed
    fn update(&mut self, value: T) -> MaybeAsync<()>;
}

/// Listener that does nothing
#[derive(Copy, Clone, Debug)]
pub struct NullListener;

impl NullListener {
    /// create a `Box<dyn Listener<T>>` that does nothing
    pub fn create<T>() -> Box<dyn Listener<T>> {
        Box::new(NullListener)
    }
}

impl<T> Listener<T> for NullListener {
    fn update(&mut self, _value: T) -> MaybeAsync<()> {
        MaybeAsync::ready(())
    }
}
