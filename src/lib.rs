use {
  std::{
    collections::HashMap,
    fmt::{self, Display, Formatter},
    hash::Hash,
  },
  thiserror::Error,
};

mod builder;
mod error;
mod machine;

pub use {builder::Builder, error::Error, machine::Machine};

type Callback<S, E, C> = Box<dyn Fn(&S, &E, &S, &mut C)>;
type Guard<S, E, C> = Box<dyn Fn(&S, &E, &C) -> bool>;

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
  (@build [$($builder:tt)*] , $from:expr, $event:expr => $to:expr, if $guard:expr $(, $($rest:tt)*)?) => {
    $crate::machine!(@build [$($builder)*.transition_if($from, $event, $to, $guard)] $(, $($rest)*)?)
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
  fn guard_blocks_transition() {
    let mut machine = machine! {
      initial: State::Foo,
      context: 0u32,
      State::Foo, Event::A => State::Bar, if |_from, _event, ctx: &u32| *ctx > 0,
    }
    .unwrap();

    assert_eq!(
      machine.send(Event::A).unwrap_err().to_string(),
      "no transition from state `Foo` on event `A`"
    );
  }

  #[test]
  fn guard_passes() {
    let mut machine = machine! {
      initial: State::Foo,
      context: 1u32,
      State::Foo, Event::A => State::Bar, if |_from, _event, ctx: &u32| *ctx > 0,
    }
    .unwrap();

    assert_eq!(machine.send(Event::A).unwrap(), &State::Bar);
  }

  #[test]
  fn guard_with_unguarded_fallback() {
    let mut machine = machine! {
      initial: State::Foo,
      context: 0u32,
      State::Foo, Event::A => State::Bar, if |_from, _event, ctx: &u32| *ctx > 0,
      State::Foo, Event::A => State::Baz,
    }
    .unwrap();

    assert_eq!(machine.send(Event::A).unwrap(), &State::Baz);

    let mut machine = machine! {
      initial: State::Foo,
      context: 1u32,
      State::Foo, Event::A => State::Bar, if |_from, _event, ctx: &u32| *ctx > 0,
      State::Foo, Event::A => State::Baz,
    }
    .unwrap();

    assert_eq!(machine.send(Event::A).unwrap(), &State::Bar);
  }

  #[test]
  fn guard_first_match_wins() {
    let mut machine = machine! {
      initial: State::Foo,
      context: 5u32,
      State::Foo, Event::A => State::Bar, if |_from, _event, ctx: &u32| *ctx > 0,
      State::Foo, Event::A => State::Baz, if |_from, _event, ctx: &u32| *ctx > 3,
    }
    .unwrap();

    assert_eq!(machine.send(Event::A).unwrap(), &State::Bar);
  }

  #[test]
  fn guard_fires_callbacks() {
    let mut machine = machine! {
      initial: State::Foo,
      context: Vec::<String>::new(),
      State::Foo, Event::A => State::Bar, if |_from, _event, _ctx: &Vec<String>| true,
      on_transition => |from, _event, to, ctx: &mut Vec<String>| {
        ctx.push(format!("{from}->{to}"));
      },
    }
    .unwrap();

    machine.send(Event::A).unwrap();

    assert_eq!(machine.context(), &["Foo->Bar"]);
  }

  #[test]
  fn can_send() {
    let machine = machine();

    assert!(machine.can_send(&Event::A));
    assert!(!machine.can_send(&Event::B));
  }

  #[test]
  fn can_send_guard() {
    #[track_caller]
    fn case(context: u32, expected: bool) {
      let machine = machine! {
        initial: State::Foo,
        context: context,
        State::Foo, Event::A => State::Bar, if |_from, _event, ctx: &u32| *ctx > 0,
      }
      .unwrap();

      assert_eq!(machine.can_send(&Event::A), expected);
    }

    case(0, false);
    case(1, true);
  }

  #[test]
  fn can_send_guard_with_fallback() {
    let machine = machine! {
      initial: State::Foo,
      context: 0u32,
      State::Foo, Event::A => State::Bar, if |_from, _event, ctx: &u32| *ctx > 0,
      State::Foo, Event::A => State::Baz,
    }
    .unwrap();

    assert!(machine.can_send(&Event::A));
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
