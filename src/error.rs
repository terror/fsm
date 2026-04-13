use super::*;

#[derive(Debug, Error)]
pub enum Error<S: Display + fmt::Debug, E: Display + fmt::Debug> {
  #[error("no context set")]
  NoContext,
  #[error("no initial state set")]
  NoInitialState,
  #[error("no transition from state `{state}` on event `{event}`")]
  NoTransition { state: S, event: E },
}
