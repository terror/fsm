use super::*;

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
  fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
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
