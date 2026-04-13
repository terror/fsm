use super::*;

pub struct Machine<S, E, C = ()> {
  pub(crate) context: C,
  pub(crate) on_enter: HashMap<S, Vec<Callback<S, E, C>>>,
  pub(crate) on_exit: HashMap<S, Vec<Callback<S, E, C>>>,
  pub(crate) on_transition: Vec<Callback<S, E, C>>,
  pub(crate) state: S,
  pub(crate) transitions: HashMap<(S, E), S>,
}

impl<S, E, C> fmt::Debug for Machine<S, E, C>
where
  S: fmt::Debug,
  E: fmt::Debug,
  C: fmt::Debug,
{
  fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
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
