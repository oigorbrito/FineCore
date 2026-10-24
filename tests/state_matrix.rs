use finecore::{Event, Execution, State};

const EVENTS: [Event; 4] = [Event::Start, Event::Succeed, Event::Fail, Event::Cancel];
const STATES: [State; 5] = [State::Ready, State::Running, State::Completed, State::Failed, State::Cancelled];

fn execution_in(state: State) -> Execution {
    let mut execution = Execution::default();
    match state {
        State::Ready => {}
        State::Running => { execution.apply(Event::Start).unwrap(); }
        State::Completed => { execution.apply(Event::Start).unwrap(); execution.apply(Event::Succeed).unwrap(); }
        State::Failed => { execution.apply(Event::Start).unwrap(); execution.apply(Event::Fail).unwrap(); }
        State::Cancelled => { execution.apply(Event::Cancel).unwrap(); }
    }
    execution
}

fn expected(state: State, event: Event) -> Option<State> {
    match (state, event) {
        (State::Ready, Event::Start) => Some(State::Running),
        (State::Ready | State::Running, Event::Cancel) => Some(State::Cancelled),
        (State::Running, Event::Succeed) => Some(State::Completed),
        (State::Running, Event::Fail) => Some(State::Failed),
        _ => None,
    }
}

#[test]
fn exhaustive_state_event_matrix() {
    let mut checked = 0;
    for state in STATES {
        for event in EVENTS {
            let mut execution = execution_in(state);
            let actual = execution.apply(event);
            match expected(state, event) {
                Some(next) => assert_eq!(actual, Ok(next), "{state:?} + {event:?}"),
                None => {
                    let error = actual.unwrap_err();
                    assert_eq!(error.state, state);
                    assert_eq!(error.event, event);
                }
            }
            assert_eq!(execution.state(), expected(state, event).unwrap_or(state));
            checked += 1;
        }
    }
    assert_eq!(checked, 20);
}
