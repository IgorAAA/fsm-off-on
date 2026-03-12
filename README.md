# fsm-off-on

A small Rust library that implements a debounced finite state machine (FSM) for `Off` / `On` input signals.

The FSM changes its internal state only after it receives **N consecutive identical signals** that are **different from the current state**. When a transition is confirmed, it sends the new signal to a queue via an abstract `QueueSender` trait.

The FSM is generic over the output channel. Any type implementing the `QueueSender` trait can be used. The project includes an example implementation for `crossbeam-channel` in the integration tests.

## Problem the FSM solves

Input streams can be noisy.
Instead of switching state on the first `On` or `Off` message, this FSM waits until the same opposite signal has been received enough times in a row.

That makes it useful as a simple debounce / confirmation layer.

## Core behavior

The FSM has two states:

- `Off`
- `On`

The FSM accepts two input signals:

- `Signal::Off`
- `Signal::On`

The FSM starts in the `Off` state.

### Transition rule

A transition happens only when:

1. The incoming signal is opposite to the current state.
2. That opposite signal is received **N times in a row**.

After that:

- the FSM changes its internal state;
- the same signal is sent to the output queue.

### Reset rule

If the FSM is currently counting an opposite signal, and a signal matching the current state arrives before the threshold is reached, the counter is reset.

In other words, the opposite signal sequence must be **strictly consecutive**.

## Example

Assume:

- initial state = `Off`
- `N = 3`

Input sequence:

```text
Off Off Off On On Off On On On
```

Processing:

- `Off Off Off` → ignored, because the FSM is already in `Off`
- `On On` → counter becomes `2`, but no transition yet
- `Off` → counter resets
- `On On On` → threshold reached, FSM switches to `On`, and `On` is sent to the queue

After that, while the FSM is in `On`, additional `On` signals do nothing.
Only `3` consecutive `Off` signals can switch it back to `Off`.

## State model

### Current state

The FSM stores:

- current state: `State::Off` or `State::On`
- threshold `N`
- current counter of consecutive opposite signals
- pending signal currently being counted
- a generic `QueueSender` implementation used as the queue output

### Output

The output queue receives messages only on confirmed state transitions:

- `On` is sent when the FSM switches from `Off` to `On`
- `Off` is sent when the FSM switches from `On` to `Off`

No other input generates queue output.

## Public API

The crate exposes:

- `Signal` — input signal enum
- `State` — FSM state enum
- `QueueSender<E>` — trait abstracting the output queue, generic over error type `E`
- `Fsm<E, Q: QueueSender<E>>` — finite state machine, generic over the error type and sender

Main methods:

- `Fsm::new(threshold, sender)` — creates a new FSM; `threshold` must be greater than `0`; `sender` is any type implementing `QueueSender<E>`
- `Fsm::handle(signal)` — processes one input signal; returns `Result<(), E>`
- `Fsm::state()` — returns the current state

### `QueueSender` trait

```rust
pub trait QueueSender<E> {
    fn send(&self, signal: Signal) -> Result<(), E>;
}
```

The error type `E` is defined by the implementor, allowing each queue backend to use its native error type. For example, the integration tests define a newtype wrapper around `crossbeam_channel::Sender<Signal>`:

```rust
struct CrossbeamSender(crossbeam_channel::Sender<Signal>);

impl QueueSender<crossbeam_channel::SendError<Signal>> for CrossbeamSender {
    fn send(&self, signal: Signal) -> Result<(), crossbeam_channel::SendError<Signal>> {
        self.0.send(signal)
    }
}
```

## Failure behavior

`Fsm::new()` panics if `threshold == 0`.

This is intentional because the task requires `N > 0`.

`Fsm::handle()` returns `Result<(), E>`. If the underlying `QueueSender::send()` fails during a state transition, the error is propagated to the caller.

## Testing

The project includes integration tests in the `tests/` folder.
They verify:

- the scenario from the task description;
- no transition when the signal matches the current state;
- exact-threshold behavior;
- counter reset on interruption;
- multiple state transitions;
- panic on invalid threshold.

Run tests with:

```bash
cargo test
```

## Dependencies

The core library has **no external dependencies**. It relies only on the Rust standard library.

The integration tests use `crossbeam-channel` (listed as a dev-dependency) to provide a concrete `QueueSender` implementation.

## Summary

This FSM is a two-state debounce mechanism:

- it ignores repeated signals equal to the current state;
- it counts only consecutive opposite signals;
- it transitions only after `N` confirmations;
- it sends a queue message only when the transition is actually performed.

