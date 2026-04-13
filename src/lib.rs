use {
  std::{
    collections::HashMap,
    fmt::{self, Display},
    hash::Hash,
  },
  thiserror::Error,
};

type Callback<S, E> = Box<dyn Fn(&S, &E, &S)>;

#[derive(Debug, Error)]
pub enum Error<S: Display + fmt::Debug, E: Display + fmt::Debug> {
  #[error("no initial state set")]
  NoInitialState,
  #[error("no transition from state `{state}` on event `{event}`")]
  NoTransition { state: S, event: E },
}

pub struct Builder<S, E> {
  initial: Option<S>,
  on_enter: HashMap<S, Vec<Callback<S, E>>>,
  on_exit: HashMap<S, Vec<Callback<S, E>>>,
  on_transition: Vec<Callback<S, E>>,
  transitions: HashMap<(S, E), S>,
}

impl<S, E> Default for Builder<S, E> {
  fn default() -> Self {
    Self {
      initial: None,
      on_enter: HashMap::new(),
      on_exit: HashMap::new(),
      on_transition: Vec::new(),
      transitions: HashMap::new(),
    }
  }
}

impl<S, E> fmt::Debug for Builder<S, E>
where
  S: fmt::Debug,
  E: fmt::Debug,
{
  fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
    f.debug_struct("Builder")
      .field("initial", &self.initial)
      .field("on_enter", &format!("[{} hooks]", self.on_enter.len()))
      .field("on_exit", &format!("[{} hooks]", self.on_exit.len()))
      .field(
        "on_transition",
        &format!("[{} hooks]", self.on_transition.len()),
      )
      .field("transitions", &self.transitions)
      .finish()
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
    Ok(Machine {
      on_enter: self.on_enter,
      on_exit: self.on_exit,
      on_transition: self.on_transition,
      state: self.initial.ok_or(Error::NoInitialState)?,
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
  pub fn on_enter(
    mut self,
    state: S,
    callback: impl Fn(&S, &E, &S) + 'static,
  ) -> Self {
    self
      .on_enter
      .entry(state)
      .or_default()
      .push(Box::new(callback));

    self
  }

  #[must_use]
  pub fn on_exit(
    mut self,
    state: S,
    callback: impl Fn(&S, &E, &S) + 'static,
  ) -> Self {
    self
      .on_exit
      .entry(state)
      .or_default()
      .push(Box::new(callback));

    self
  }

  #[must_use]
  pub fn on_transition(
    mut self,
    callback: impl Fn(&S, &E, &S) + 'static,
  ) -> Self {
    self.on_transition.push(Box::new(callback));
    self
  }

  #[must_use]
  pub fn transition(mut self, from: S, event: E, to: S) -> Self {
    self.transitions.insert((from, event), to);
    self
  }
}

pub struct Machine<S, E> {
  on_enter: HashMap<S, Vec<Callback<S, E>>>,
  on_exit: HashMap<S, Vec<Callback<S, E>>>,
  on_transition: Vec<Callback<S, E>>,
  state: S,
  transitions: HashMap<(S, E), S>,
}

impl<S, E> fmt::Debug for Machine<S, E>
where
  S: fmt::Debug,
  E: fmt::Debug,
{
  fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
    f.debug_struct("Machine")
      .field("on_enter", &format!("[{} hooks]", self.on_enter.len()))
      .field("on_exit", &format!("[{} hooks]", self.on_exit.len()))
      .field(
        "on_transition",
        &format!("[{} hooks]", self.on_transition.len()),
      )
      .field("state", &self.state)
      .field("transitions", &self.transitions)
      .finish()
  }
}

impl<S, E> Machine<S, E>
where
  S: Clone + Eq + Hash + Display + fmt::Debug,
  E: Clone + Eq + Hash + Display + fmt::Debug,
{
  /// # Errors
  ///
  /// Returns `Error::NoTransition` if no transition exists for the
  /// current state and event.
  pub fn send(&mut self, event: E) -> Result<&S, Error<S, E>> {
    let from = self.state.clone();

    let Some(to) = self
      .transitions
      .get(&(from.clone(), event.clone()))
      .cloned()
    else {
      return Err(Error::NoTransition { state: from, event });
    };

    if let Some(hooks) = self.on_exit.get(&from) {
      for hook in hooks {
        hook(&from, &event, &to);
      }
    }

    for hook in &self.on_transition {
      hook(&from, &event, &to);
    }

    if let Some(hooks) = self.on_enter.get(&to) {
      for hook in hooks {
        hook(&from, &event, &to);
      }
    }

    self.state = to;

    Ok(&self.state)
  }

  pub fn state(&self) -> &S {
    &self.state
  }
}

#[macro_export]
macro_rules! machine {
  (
    initial: $initial:expr,
    $($from:expr, $event:expr => $to:expr),+
    $(,)?
  ) => {
    $crate::Builder::new()
      .initial($initial)
      $(.transition($from, $event, $to))+
      .build()
  };
}

#[cfg(test)]
mod tests {
  use {
    super::*,
    std::{cell::Cell, rc::Rc},
  };

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
    machine! {
      initial: State::Foo,
      State::Foo, Event::A => State::Bar,
      State::Bar, Event::B => State::Baz,
      State::Baz, Event::A => State::Foo,
    }
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
    let mut machine = machine! {
      initial: State::Foo,
      State::Foo, Event::A => State::Bar,
      State::Foo, Event::A => State::Baz,
    }
    .unwrap();

    assert_eq!(machine.send(Event::A).unwrap(), &State::Baz);
  }

  #[test]
  fn self_transition() {
    let mut machine = machine! {
      initial: State::Foo,
      State::Foo, Event::A => State::Foo,
    }
    .unwrap();

    assert_eq!(machine.send(Event::A).unwrap(), &State::Foo);
    assert_eq!(machine.send(Event::A).unwrap(), &State::Foo);
  }

  #[test]
  fn on_enter() {
    let count = Rc::new(Cell::new(0));
    let counter = count.clone();

    let mut machine = Builder::new()
      .initial(State::Foo)
      .transition(State::Foo, Event::A, State::Bar)
      .transition(State::Bar, Event::B, State::Baz)
      .on_enter(State::Bar, move |_from, _event, _to| {
        counter.set(counter.get() + 1);
      })
      .build()
      .unwrap();

    machine.send(Event::A).unwrap();
    assert_eq!(count.get(), 1);

    machine.send(Event::B).unwrap();
    assert_eq!(count.get(), 1);
  }

  #[test]
  fn on_exit() {
    let count = Rc::new(Cell::new(0));
    let counter = count.clone();

    let mut machine = Builder::new()
      .initial(State::Foo)
      .transition(State::Foo, Event::A, State::Bar)
      .transition(State::Bar, Event::B, State::Baz)
      .on_exit(State::Foo, move |_from, _event, _to| {
        counter.set(counter.get() + 1);
      })
      .build()
      .unwrap();

    machine.send(Event::A).unwrap();
    assert_eq!(count.get(), 1);

    machine.send(Event::B).unwrap();
    assert_eq!(count.get(), 1);
  }

  #[test]
  fn on_transition() {
    let count = Rc::new(Cell::new(0));
    let counter = count.clone();

    let mut machine = Builder::new()
      .initial(State::Foo)
      .transition(State::Foo, Event::A, State::Bar)
      .transition(State::Bar, Event::B, State::Baz)
      .on_transition(move |_from, _event, _to| {
        counter.set(counter.get() + 1);
      })
      .build()
      .unwrap();

    machine.send(Event::A).unwrap();
    assert_eq!(count.get(), 1);

    machine.send(Event::B).unwrap();
    assert_eq!(count.get(), 2);
  }

  #[test]
  fn callback_order() {
    let log = Rc::new(Cell::new(Vec::new()));

    let push = |log: &Rc<Cell<Vec<&'static str>>>, tag: &'static str| {
      let mut v = log.take();
      v.push(tag);
      log.set(v);
    };

    let l = log.clone();
    let on_exit = move |_: &State, _: &Event, _: &State| push(&l, "exit");

    let l = log.clone();
    let on_transition =
      move |_: &State, _: &Event, _: &State| push(&l, "transition");

    let l = log.clone();
    let on_enter = move |_: &State, _: &Event, _: &State| push(&l, "enter");

    let mut machine = Builder::new()
      .initial(State::Foo)
      .transition(State::Foo, Event::A, State::Bar)
      .on_exit(State::Foo, on_exit)
      .on_transition(on_transition)
      .on_enter(State::Bar, on_enter)
      .build()
      .unwrap();

    machine.send(Event::A).unwrap();

    assert_eq!(log.take(), vec!["exit", "transition", "enter"]);
  }

  #[test]
  fn callback_receives_context() {
    let log = Rc::new(Cell::new(Vec::new()));
    let l = log.clone();

    let mut machine = Builder::new()
      .initial(State::Foo)
      .transition(State::Foo, Event::A, State::Bar)
      .on_transition(move |from, event, to| {
        let mut v = l.take();
        v.push(format!("{from}+{event}=>{to}"));
        l.set(v);
      })
      .build()
      .unwrap();

    machine.send(Event::A).unwrap();

    assert_eq!(log.take(), vec!["Foo+A=>Bar"]);
  }

  #[test]
  fn no_callbacks_on_failed_transition() {
    let count = Rc::new(Cell::new(0));
    let counter = count.clone();

    let mut machine = Builder::new()
      .initial(State::Foo)
      .transition(State::Foo, Event::A, State::Bar)
      .on_transition(move |_from, _event, _to| {
        counter.set(counter.get() + 1);
      })
      .build()
      .unwrap();

    let _ = machine.send(Event::B);

    assert_eq!(count.get(), 0);
  }

  #[test]
  fn self_transition_fires_callbacks() {
    let count = Rc::new(Cell::new(0));
    let counter = count.clone();

    let mut machine = Builder::new()
      .initial(State::Foo)
      .transition(State::Foo, Event::A, State::Foo)
      .on_enter(State::Foo, move |_from, _event, _to| {
        counter.set(counter.get() + 1);
      })
      .build()
      .unwrap();

    machine.send(Event::A).unwrap();

    assert_eq!(count.get(), 1);
  }
}
