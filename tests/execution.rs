use finecore::{Event, Execution, State};

#[test]
fn successful_execution() {
    let mut e = Execution::default();
    assert_eq!(e.apply(Event::Start), Ok(State::Running));
    assert_eq!(e.apply(Event::Succeed), Ok(State::Completed));
}

#[test]
fn terminal_states_reject_transitions() {
    let mut e = Execution::default();
    assert_eq!(e.apply(Event::Cancel), Ok(State::Cancelled));
    assert!(e.apply(Event::Start).is_err());
    assert_eq!(e.state(), State::Cancelled);
}

#[test]
fn cannot_complete_without_starting() {
    let mut e = Execution::default();
    assert!(e.apply(Event::Succeed).is_err());
    assert_eq!(e.state(), State::Ready);
}
