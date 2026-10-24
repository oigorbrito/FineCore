//! FineCore: independent execution-state kernel.
//! No external dependencies, providers, networking or donor code.

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum State {
    Ready,
    Running,
    Completed,
    Failed,
    Cancelled,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Event {
    Start,
    Succeed,
    Fail,
    Cancel,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct InvalidTransition {
    pub state: State,
    pub event: Event,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Execution {
    state: State,
}

impl Default for Execution {
    fn default() -> Self {
        Self {
            state: State::Ready,
        }
    }
}

impl Execution {
    pub fn state(&self) -> State {
        self.state
    }

    pub fn apply(&mut self, event: Event) -> Result<State, InvalidTransition> {
        use Event::*;
        use State::*;
        let next = match (self.state, event) {
            (Ready, Start) => Running,
            (Ready | Running, Cancel) => Cancelled,
            (Running, Succeed) => Completed,
            (Running, Fail) => Failed,
            _ => {
                return Err(InvalidTransition {
                    state: self.state,
                    event,
                })
            }
        };
        self.state = next;
        Ok(next)
    }
}
