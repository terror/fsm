use {
  std::{
    collections::HashMap,
    fmt::{self, Display},
    hash::Hash,
  },
  thiserror::Error,
};

mod error;

pub use error::Error;

type Callback<S, E, C> = Box<dyn Fn(&S, &E, &S, &mut C)>;

pub struct Builder<S, E, C = ()> {
  context: Option<C>,
  initial: Option<S>,
  on_enter: HashMap<S, Vec<Callback<S, E, C>>>,
  on_exit: HashMap<S, Vec<Callback<S, E, C>>>,
  on_transition: Vec<Callback<S, E, C>>,
  transitions: HashMap<(S, E), S>,
}

impl<S, E, C: Default> Default for Builder<S, E, C> {
  fn default() -> Self {
    Self {
      context: Some(C::default()),
      initial: None,
      on_enter: HashMap::new(),
      on_exit: HashMap::new(),
      on_transition: Vec::new(),
      transitions: HashMap::new(),
    }
  }
}

impl<S, E, C> fmt::Debug for Builder<S, E, C>
where
  S: fmt::Debug,
  E: fmt::Debug,
  C: fmt::Debug,
{
  fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
    f.debug_struct("Builder")
      .field("context", &self.context)
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

impl<S, E, C> Builder<S, E, C>
where
  S: Clone + Eq + Hash + Display + fmt::Debug,
  E: Eq + Hash + Display + fmt::Debug,
{
  /// # Errors
  ///
  /// Returns `Error::NoContext` if no context was set.
  /// Returns `Error::NoInitialState` if no initial state was set.
  pub fn build(self) -> Result<Machine<S, E, C>, Error<S, E>> {
    Ok(Machine {
      context: self.context.ok_or(Error::NoContext)?,
      on_enter: self.on_enter,
      on_exit: self.on_exit,
      on_transition: self.on_transition,
      state: self.initial.ok_or(Error::NoInitialState)?,
      transitions: self.transitions,
    })
  }

  #[must_use]
  pub fn context(mut self, context: C) -> Self {
    self.context = Some(context);
    self
  }

  #[must_use]
  pub fn initial(mut self, state: S) -> Self {
    self.initial = Some(state);
    self
  }

  #[must_use]
  pub fn new() -> Self
  where
    C: Default,
  {
    Self::default()
  }

  #[must_use]
  pub fn on_enter(
    mut self,
    state: S,
    callback: impl Fn(&S, &E, &S, &mut C) + 'static,
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
    callback: impl Fn(&S, &E, &S, &mut C) + 'static,
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
    callback: impl Fn(&S, &E, &S, &mut C) + 'static,
  ) -> Self {
    self.on_transition.push(Box::new(callback));
    self
  }

  #[must_use]
  pub fn transition(mut self, from: S, event: E, to: S) -> Self {
    self.transitions.insert((from, event), to);
    self
  }

  #[must_use]
  pub fn with_context(context: C) -> Self {
    Self {
      context: Some(context),
      initial: None,
      on_enter: HashMap::new(),
      on_exit: HashMap::new(),
      on_transition: Vec::new(),
      transitions: HashMap::new(),
    }
  }
}

pub struct Machine<S, E, C = ()> {
  context: C,
  on_enter: HashMap<S, Vec<Callback<S, E, C>>>,
  on_exit: HashMap<S, Vec<Callback<S, E, C>>>,
  on_transition: Vec<Callback<S, E, C>>,
  state: S,
  transitions: HashMap<(S, E), S>,
}

impl<S, E, C> fmt::Debug for Machine<S, E, C>
where
  S: fmt::Debug,
  E: fmt::Debug,
  C: fmt::Debug,
{
  fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
    f.debug_struct("Machine")
      .field("context", &self.context)
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

impl<S, E, C> Machine<S, E, C>
where
  S: Clone + Eq + Hash + Display + fmt::Debug,
  E: Clone + Eq + Hash + Display + fmt::Debug,
{
  pub fn context(&self) -> &C {
    &self.context
  }

  pub fn context_mut(&mut self) -> &mut C {
    &mut self.context
  }

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
        hook(&from, &event, &to, &mut self.context);
      }
    }

    for hook in &self.on_transition {
      hook(&from, &event, &to, &mut self.context);
    }

    if let Some(hooks) = self.on_enter.get(&to) {
      for hook in hooks {
        hook(&from, &event, &to, &mut self.context);
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
  (@build [$($builder:tt)*]) => {
    $($builder)*.build()
  };
  (@build [$($builder:tt)*] ,) => {
    $($builder)*.build()
  };
  (@build [$($builder:tt)*] , on_enter $state:expr => $callback:expr $(, $($rest:tt)*)?) => {
    $crate::machine!(@build [$($builder)*.on_enter($state, $callback)] $(, $($rest)*)?)
  };
  (@build [$($builder:tt)*] , on_exit $state:expr => $callback:expr $(, $($rest:tt)*)?) => {
    $crate::machine!(@build [$($builder)*.on_exit($state, $callback)] $(, $($rest)*)?)
  };
  (@build [$($builder:tt)*] , on_transition => $callback:expr $(, $($rest:tt)*)?) => {
    $crate::machine!(@build [$($builder)*.on_transition($callback)] $(, $($rest)*)?)
  };
  (@build [$($builder:tt)*] , $from:expr, $event:expr => $to:expr $(, $($rest:tt)*)?) => {
    $crate::machine!(@build [$($builder)*.transition($from, $event, $to)] $(, $($rest)*)?)
  };
  (initial: $initial:expr, context: $context:expr, $($rest:tt)*) => {
    $crate::machine!(@build [$crate::Builder::with_context($context).initial($initial)], $($rest)*)
  };
  (initial: $initial:expr, context: $context:expr $(,)?) => {
    $crate::machine!(@build [$crate::Builder::with_context($context).initial($initial)])
  };
  (initial: $initial:expr, $($rest:tt)*) => {
    $crate::machine!(@build [$crate::Builder::new().initial($initial)], $($rest)*)
  };
  (initial: $initial:expr $(,)?) => {
    $crate::machine!(@build [$crate::Builder::new().initial($initial)])
  };
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
    machine! {
      initial: State::Foo,
      State::Foo, Event::A => State::Bar,
      State::Bar, Event::B => State::Baz,
      State::Baz, Event::A => State::Foo,
    }
    .unwrap()
  }

  #[test]
  fn callback_order() {
    let on_exit = |_: &State, _: &Event, _: &State, ctx: &mut Vec<&str>| {
      ctx.push("exit");
    };

    let on_transition =
      |_: &State, _: &Event, _: &State, ctx: &mut Vec<&str>| {
        ctx.push("transition");
      };

    let on_enter = |_: &State, _: &Event, _: &State, ctx: &mut Vec<&str>| {
      ctx.push("enter");
    };

    let mut machine = machine! {
      initial: State::Foo,
      context: Vec::<&str>::new(),
      State::Foo, Event::A => State::Bar,
      on_exit State::Foo => on_exit,
      on_transition => on_transition,
      on_enter State::Bar => on_enter,
    }
    .unwrap();

    machine.send(Event::A).unwrap();

    assert_eq!(machine.context(), &["exit", "transition", "enter"]);
  }

  #[test]
  fn callback_receives_args() {
    let mut machine = machine! {
      initial: State::Foo,
      context: Vec::<String>::new(),
      State::Foo, Event::A => State::Bar,
      on_transition => |from, event, to, ctx: &mut Vec<String>| {
        ctx.push(format!("{from}+{event}=>{to}"));
      },
    }
    .unwrap();

    machine.send(Event::A).unwrap();

    assert_eq!(machine.context(), &["Foo+A=>Bar"]);
  }

  #[test]
  fn context_mut() {
    let mut machine = machine! {
      initial: State::Foo,
      context: 0u32,
      State::Foo, Event::A => State::Bar,
    }
    .unwrap();

    *machine.context_mut() = 99;
    assert_eq!(*machine.context(), 99);
  }

  #[test]
  fn initial_state() {
    assert_eq!(machine().state(), &State::Foo);
  }

  #[test]
  fn macro_context_only() {
    let machine: Machine<State, Event, u32> = machine! {
      initial: State::Foo,
      context: 42u32,
    }
    .unwrap();

    assert_eq!(machine.state(), &State::Foo);
    assert_eq!(*machine.context(), 42);
  }

  #[test]
  fn macro_initial_only() {
    let machine: Machine<State, Event> = machine! {
      initial: State::Foo,
    }
    .unwrap();

    assert_eq!(machine.state(), &State::Foo);
  }

  #[test]
  fn macro_transitions_only() {
    let mut machine: Machine<State, Event> = machine! {
      initial: State::Foo,
      State::Foo, Event::A => State::Bar,
    }
    .unwrap();

    assert_eq!(machine.send(Event::A).unwrap(), &State::Bar);
  }

  #[test]
  fn no_callbacks_on_failed_transition() {
    let mut machine = machine! {
      initial: State::Foo,
      context: 0u32,
      State::Foo, Event::A => State::Bar,
      on_transition => |_from, _event, _to, ctx: &mut u32| {
        *ctx += 1;
      },
    }
    .unwrap();

    let _ = machine.send(Event::B);

    assert_eq!(*machine.context(), 0);
  }

  #[test]
  fn no_context() {
    assert_eq!(
      Builder::<State, Event, String>::with_context(String::new())
        .build()
        .unwrap_err()
        .to_string(),
      "no initial state set"
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
  fn no_transition() {
    assert_eq!(
      machine().send(Event::B).unwrap_err().to_string(),
      "no transition from state `Foo` on event `B`"
    );
  }

  #[test]
  fn on_enter() {
    let mut machine = machine! {
      initial: State::Foo,
      context: 0u32,
      State::Foo, Event::A => State::Bar,
      State::Bar, Event::B => State::Baz,
      on_enter State::Bar => |_from, _event, _to, ctx: &mut u32| {
        *ctx += 1;
      },
    }
    .unwrap();

    machine.send(Event::A).unwrap();
    assert_eq!(*machine.context(), 1);

    machine.send(Event::B).unwrap();
    assert_eq!(*machine.context(), 1);
  }

  #[test]
  fn on_exit() {
    let mut machine = machine! {
      initial: State::Foo,
      context: 0u32,
      State::Foo, Event::A => State::Bar,
      State::Bar, Event::B => State::Baz,
      on_exit State::Foo => |_from, _event, _to, ctx: &mut u32| {
        *ctx += 1;
      },
    }
    .unwrap();

    machine.send(Event::A).unwrap();
    assert_eq!(*machine.context(), 1);

    machine.send(Event::B).unwrap();
    assert_eq!(*machine.context(), 1);
  }

  #[test]
  fn on_transition() {
    let mut machine = machine! {
      initial: State::Foo,
      context: 0u32,
      State::Foo, Event::A => State::Bar,
      State::Bar, Event::B => State::Baz,
      on_transition => |_from, _event, _to, ctx: &mut u32| {
        *ctx += 1;
      },
    }
    .unwrap();

    machine.send(Event::A).unwrap();
    assert_eq!(*machine.context(), 1);

    machine.send(Event::B).unwrap();
    assert_eq!(*machine.context(), 2);
  }

  #[test]
  fn self_transition() {
    let mut machine: Machine<State, Event> = machine! {
      initial: State::Foo,
      State::Foo, Event::A => State::Foo,
    }
    .unwrap();

    assert_eq!(machine.send(Event::A).unwrap(), &State::Foo);
    assert_eq!(machine.send(Event::A).unwrap(), &State::Foo);
  }

  #[test]
  fn self_transition_fires_callbacks() {
    let mut machine = machine! {
      initial: State::Foo,
      context: 0u32,
      State::Foo, Event::A => State::Foo,
      on_enter State::Foo => |_from, _event, _to, ctx: &mut u32| {
        *ctx += 1;
      },
    }
    .unwrap();

    machine.send(Event::A).unwrap();

    assert_eq!(*machine.context(), 1);
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
  fn send_transitions_state() {
    let mut machine = machine();

    assert_eq!(machine.send(Event::A).unwrap(), &State::Bar);
    assert_eq!(machine.state(), &State::Bar);
  }

  #[test]
  fn transition_overwrites() {
    let mut machine: Machine<State, Event> = machine! {
      initial: State::Foo,
      State::Foo, Event::A => State::Bar,
      State::Foo, Event::A => State::Baz,
    }
    .unwrap();

    assert_eq!(machine.send(Event::A).unwrap(), &State::Baz);
  }

  #[test]
  fn with_context() {
    let mut machine = machine! {
      initial: State::Foo,
      context: Vec::<String>::new(),
      State::Foo, Event::A => State::Bar,
      on_transition => |from, _event, to, ctx: &mut Vec<String>| {
        ctx.push(format!("{from}->{to}"));
      },
    }
    .unwrap();

    machine.send(Event::A).unwrap();

    assert_eq!(machine.context(), &["Foo->Bar"]);
  }
}
