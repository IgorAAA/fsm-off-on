use crossbeam_channel;

use fsm_off_on::{Fsm, QueueSender, Signal, State};

/// Newtype wrapper around `crossbeam_channel::Sender<Signal>`
/// to satisfy the orphan rule when implementing `QueueSender`.
struct CrossbeamSender(crossbeam_channel::Sender<Signal>);

impl QueueSender<crossbeam_channel::SendError<Signal>> for CrossbeamSender {
    fn send(&self, signal: Signal) -> Result<(), crossbeam_channel::SendError<Signal>> {
        self.0.send(signal)
    }
}

#[test]
fn scenario_from_task_description() {
    let (tx, rx) = crossbeam_channel::unbounded::<Signal>();

    let mut fsm = Fsm::new(3, CrossbeamSender(tx));

    // Scenario from the task description:
    let signals = vec![
        // Many Off signals while initial state is Off — nothing happens
        Signal::Off, Signal::Off, Signal::Off, Signal::Off,
        // On twice — not enough (need 3)
        Signal::On, Signal::On,
        // Off — resets the counter
        Signal::Off,
        // On 3 times in a row — transition to On, send On to queue
        Signal::On, Signal::On, Signal::On,
        // More On — nothing happens, already in On state
        Signal::On, Signal::On,
        // Off twice — not enough
        Signal::Off, Signal::Off,
        // On — resets the counter
        Signal::On,
        // Off 3 times — transition to Off, send Off to queue
        Signal::Off, Signal::Off, Signal::Off,
    ];

    for signal in &signals {
        fsm.handle(*signal).unwrap();
    }

    // Close the sender so the receiver iterator terminates
    drop(fsm);

    let messages: Vec<Signal> = rx.iter().collect();
    assert_eq!(messages, vec![Signal::On, Signal::Off]);
}

#[test]
fn no_transition_when_signal_matches_current_state() {
    let (tx, rx) = crossbeam_channel::unbounded::<Signal>();
    let mut fsm = Fsm::new(2, CrossbeamSender(tx));

    // Initial state is Off; sending Off signals should produce no output
    for _ in 0..10 {
        fsm.handle(Signal::Off).unwrap();
    }

    assert_eq!(fsm.state(), State::Off);
    drop(fsm);

    let messages: Vec<Signal> = rx.iter().collect();
    assert!(messages.is_empty());
}

#[test]
fn transition_at_exact_threshold() {
    let (tx, rx) = crossbeam_channel::unbounded::<Signal>();
    let mut fsm = Fsm::new(1, CrossbeamSender(tx));

    // With N=1, a single opposite signal should trigger a transition
    fsm.handle(Signal::On).unwrap();
    assert_eq!(fsm.state(), State::On);

    fsm.handle(Signal::Off).unwrap();
    assert_eq!(fsm.state(), State::Off);

    drop(fsm);

    let messages: Vec<Signal> = rx.iter().collect();
    assert_eq!(messages, vec![Signal::On, Signal::Off]);
}

#[test]
fn counter_resets_on_interruption() {
    let (tx, rx) = crossbeam_channel::unbounded::<Signal>();
    let mut fsm = Fsm::new(3, CrossbeamSender(tx));

    // Send On twice, then Off to interrupt, repeat — no transition
    fsm.handle(Signal::On).unwrap();
    fsm.handle(Signal::On).unwrap();
    fsm.handle(Signal::Off).unwrap(); // reset
    fsm.handle(Signal::On).unwrap();
    fsm.handle(Signal::On).unwrap();
    fsm.handle(Signal::Off).unwrap(); // reset

    assert_eq!(fsm.state(), State::Off);
    drop(fsm);

    let messages: Vec<Signal> = rx.iter().collect();
    assert!(messages.is_empty());
}

#[test]
fn multiple_transitions() {
    let (tx, rx) = crossbeam_channel::unbounded::<Signal>();
    let mut fsm = Fsm::new(2, CrossbeamSender(tx));

    // Off -> On
    fsm.handle(Signal::On).unwrap();
    fsm.handle(Signal::On).unwrap();
    assert_eq!(fsm.state(), State::On);

    // On -> Off
    fsm.handle(Signal::Off).unwrap();
    fsm.handle(Signal::Off).unwrap();
    assert_eq!(fsm.state(), State::Off);

    // Off -> On again
    fsm.handle(Signal::On).unwrap();
    fsm.handle(Signal::On).unwrap();
    assert_eq!(fsm.state(), State::On);

    drop(fsm);

    let messages: Vec<Signal> = rx.iter().collect();
    assert_eq!(messages, vec![Signal::On, Signal::Off, Signal::On]);
}

#[test]
#[should_panic(expected = "N must be > 0")]
fn threshold_zero_panics() {
    let (tx, _rx) = crossbeam_channel::unbounded::<Signal>();
    let _fsm = Fsm::new(0, CrossbeamSender(tx));
}

