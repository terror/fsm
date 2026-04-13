use {
  std::{
    collections::HashMap,
    fmt::{self, Display},
    hash::Hash,
  },
  thiserror::Error,
};

#[derive(Debug, Error)]
pub enum Error<S: Display + fmt::Debug, E: Display + fmt::Debug> {
  #[error("no initial state set")]
  NoInitialState,
  #[error("no transition from state `{state}` on event `{event}`")]
  NoTransition { state: S, event: E },
}

#[derive(Debug)]
pub struct Builder<S, E> {
  initial: Option<S>,
  transitions: HashMap<(S, E), S>,
}

impl<S, E> Default for Builder<S, E> {
  fn default() -> Self {
    Self {
      initial: None,
      transitions: HashMap::new(),
    }
  }
}

impl<S, E> Builder<S, E>
where
  S: Clone + Eq + Hash + Display + fmt::Debug,
  E: Eq + Hash + Display + fmt::Debug,
{
  /// # Errors
  ///
  /// Returns `Error::NoInitialState` if no initial state was set.
  pub fn build(self) -> Result<Machine<S, E>, Error<S, E>> {
    let state = self.initial.ok_or(Error::NoInitialState)?;

    Ok(Machine {
      state,
      transitions: self.transitions,
    })
  }

  #[must_use]
  pub fn initial(mut self, state: S) -> Self {
    self.initial = Some(state);
    self
  }

  #[must_use]
  pub fn new() -> Self {
    Self::default()
  }

  #[must_use]
  pub fn transition(mut self, from: S, event: E, to: S) -> Self {
    self.transitions.insert((from, event), to);
    self
  }
}

#[derive(Debug)]
pub struct Machine<S, E> {
  state: S,
  transitions: HashMap<(S, E), S>,
}

impl<S, E> Machine<S, E>
where
  S: Clone + Eq + Hash + Display + fmt::Debug,
  E: Eq + Hash + Display + fmt::Debug,
{
  /// # Errors
  ///
  /// Returns `Error::NoTransition` if no transition exists for the
  /// current state and event.
  pub fn send(&mut self, event: E) -> Result<&S, Error<S, E>> {
    let key = (self.state.clone(), event);

    let to = self.transitions.get(&key).ok_or_else(|| {
      let (state, event) = key;
      Error::NoTransition { state, event }
    })?;

    self.state = to.clone();

    Ok(&self.state)
  }

  pub fn state(&self) -> &S {
    &self.state
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  #[derive(Clone, Debug, Eq, Hash, PartialEq, derive_more::Display)]
  enum State {
    Bar,
    Baz,
    Foo,
  }

  #[derive(Clone, Debug, Eq, Hash, PartialEq, derive_more::Display)]
  enum Event {
    A,
    B,
  }

  fn machine() -> Machine<State, Event> {
    Builder::new()
      .initial(State::Foo)
      .transition(State::Foo, Event::A, State::Bar)
      .transition(State::Bar, Event::B, State::Baz)
      .transition(State::Baz, Event::A, State::Foo)
      .build()
      .unwrap()
  }

  #[test]
  fn initial_state() {
    assert_eq!(machine().state(), &State::Foo);
  }

  #[test]
  fn send_transitions_state() {
    let mut machine = machine();

    assert_eq!(machine.send(Event::A).unwrap(), &State::Bar);
    assert_eq!(machine.state(), &State::Bar);
  }

  #[test]
  fn send_chain() {
    let mut machine = machine();

    machine.send(Event::A).unwrap();
    machine.send(Event::B).unwrap();
    machine.send(Event::A).unwrap();

    assert_eq!(machine.state(), &State::Foo);
  }

  #[test]
  fn no_transition() {
    assert_eq!(
      machine().send(Event::B).unwrap_err().to_string(),
      "no transition from state `Foo` on event `B`"
    );
  }

  #[test]
  fn no_initial_state() {
    assert_eq!(
      Builder::<State, Event>::new()
        .build()
        .unwrap_err()
        .to_string(),
      "no initial state set"
    );
  }

  #[test]
  fn transition_overwrites() {
    let mut machine = Builder::new()
      .initial(State::Foo)
      .transition(State::Foo, Event::A, State::Bar)
      .transition(State::Foo, Event::A, State::Baz)
      .build()
      .unwrap();

    assert_eq!(machine.send(Event::A).unwrap(), &State::Baz);
  }

  #[test]
  fn self_transition() {
    let mut machine = Builder::new()
      .initial(State::Foo)
      .transition(State::Foo, Event::A, State::Foo)
      .build()
      .unwrap();

    assert_eq!(machine.send(Event::A).unwrap(), &State::Foo);
    assert_eq!(machine.send(Event::A).unwrap(), &State::Foo);
  }
}
