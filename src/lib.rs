use std::marker::PhantomData;
use std::num::NonZeroUsize;

/// Input signal
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Signal {
    Off,
    On,
}

/// Finite state machine state
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum State {
    Off,
    On,
}

impl State {
    /// Returns the signal corresponding to the opposite state
    fn opposite_signal(&self) -> Signal {
        match self {
            State::Off => Signal::On,
            State::On => Signal::Off,
        }
    }

    /// Toggles the state
    fn toggle(&self) -> State {
        match self {
            State::Off => State::On,
            State::On => State::Off,
        }
    }
}

/// Abstraction over the output queue.
/// Any type that can send a `Signal` can be used as the FSM's output.
pub trait QueueSender<E> {
    /// Sends a signal to the queue.
    /// Returns `Err` with a description if the send fails.
    fn send(&self, signal: Signal) -> Result<(), E>;
}

/// Debounced FSM: transitions state only after N consecutive
/// identical signals that differ from the current state.
pub struct Fsm<E, Q: QueueSender<E>> {
    state: State,
    threshold: NonZeroUsize, // N — number of consecutive signals required for transition
    counter: usize,          // current counter of consecutive matching signals
    pending: Option<Signal>, // which signal we are currently counting
    queue_tx: Q,
    _phantom: PhantomData<E>,
}

impl<E, Q: QueueSender<E>> Fsm<E, Q> {
    pub fn new(threshold: NonZeroUsize, queue_tx: Q) -> Self {
        Fsm {
            state: State::Off,
            threshold,
            counter: 0,
            pending: None,
            queue_tx,
            _phantom: PhantomData,
        }
    }

    /// Returns the current state of the FSM
    pub fn state(&self) -> State {
        self.state
    }

    /// Processes an incoming signal
    pub fn handle(&mut self, signal: Signal) -> Result<(), E> {
        let target_signal = self.state.opposite_signal();

        if signal == target_signal {
            // Signal differs from the current state — count it
            if self.pending == Some(signal) {
                self.counter += 1;
            } else {
                // Start counting a new series
                self.pending = Some(signal);
                self.counter = 1;
            }

            if self.counter >= self.threshold.get() {
                // Transition state and send a message to the queue
                self.state = self.state.toggle();
                self.counter = 0;
                self.pending = None;

                self.queue_tx.send(signal)?;
            }

            Ok(())
        } else {
            // Signal matches the current state — reset the counter
            self.counter = 0;
            self.pending = None;
            Ok(())
        }
    }
}
