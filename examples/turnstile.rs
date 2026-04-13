use fsm::{machine, Machine};
use std::fmt::{self, Display, Formatter};

#[derive(Clone, Debug, Eq, Hash, PartialEq)]
enum State {
  Locked,
  Unlocked,
}

impl Display for State {
  fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
    match self {
      State::Locked => write!(f, "locked"),
      State::Unlocked => write!(f, "unlocked"),
    }
  }
}

#[derive(Clone, Debug, Eq, Hash, PartialEq)]
enum Event {
  Coin,
  Push,
}

impl Display for Event {
  fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
    match self {
      Event::Coin => write!(f, "coin"),
      Event::Push => write!(f, "push"),
    }
  }
}

#[derive(Debug)]
struct Context {
  coins: u32,
  entries: u32,
  rejected_pushes: u32,
}

fn main() {
  let mut machine: Machine<State, Event, Context> = machine! {
    initial: State::Locked,
    context: Context { coins: 0, entries: 0, rejected_pushes: 0 },

    State::Locked, Event::Coin => State::Unlocked,
    State::Locked, Event::Push => State::Locked,
    State::Unlocked, Event::Push => State::Locked,
    State::Unlocked, Event::Coin => State::Unlocked,

    on_enter State::Unlocked => |_from, _event, _to, ctx: &mut Context| {
      ctx.coins += 1;
      println!("  coin accepted, turnstile unlocked");
    },

    on_exit State::Unlocked => |_from, _event, _to, ctx: &mut Context| {
      ctx.entries += 1;
      println!("  person walked through, turnstile locked");
    },

    on_transition => |from, event, to, _ctx: &mut Context| {
      println!("[{from}] --{event}--> [{to}]");
    },
  }
  .unwrap();

  println!("turnstile fsm\n");
  println!("state: {}\n", machine.state());

  println!("push while locked:");
  if !machine.can_send(&Event::Push) || *machine.state() == State::Locked {
    machine.context_mut().rejected_pushes += 1;
    println!("  push rejected (locked)\n");
  }

  println!("insert coin:");
  machine.send(Event::Coin).unwrap();
  println!("  can push? {}\n", machine.can_send(&Event::Push));

  println!("push while unlocked:");
  machine.send(Event::Push).unwrap();
  println!();

  println!("insert coin and push again:");
  machine.send(Event::Coin).unwrap();
  machine.send(Event::Push).unwrap();
  println!();

  let ctx = machine.context();
  println!("stats:");
  println!("  coins collected:  {}", ctx.coins);
  println!("  people entered:   {}", ctx.entries);
  println!("  rejected pushes:  {}", ctx.rejected_pushes);
  println!("  final state:      {}", machine.state());
}
