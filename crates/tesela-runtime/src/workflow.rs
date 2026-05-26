//! Generic state machine (workflow engine).
//!
//! A [`Machine`] owns a set of [`TransitionDef`]s indexed by
//! `(from_state, event)`.  Calling [`Machine::transition`] enforces the
//! guard function and returns the new state on success.
//!
//! Machines are pure data structures — they carry no mutable state.
//! Callers own the "current state" value and pass it to `transition`.

use tesela_core::Error;
use std::collections::HashMap;

// ---------------------------------------------------------------------------
// Types
// ---------------------------------------------------------------------------

/// An opaque state label.  Use string constants in your domain module.
#[derive(Debug, Clone, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
pub struct State(pub String);

impl State {
    /// Create a new state from a static string.
    pub fn new(s: impl Into<String>) -> Self {
        Self(s.into())
    }
}

impl std::fmt::Display for State {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.0)
    }
}

/// Guard function signature — returns `Ok(())` to allow or `Err` to block.
pub type Guard =
    fn(data: &std::collections::BTreeMap<String, tesela_core::Value>) -> Result<(), Error>;

/// A single allowed state transition.
#[derive(Clone)]
pub struct TransitionDef {
    /// Source state.
    pub from: State,
    /// Destination state.
    pub to: State,
    /// Event name that triggers this transition.
    pub event: String,
    /// Optional guard — called before the transition is committed.
    pub guard: Option<Guard>,
}

// ---------------------------------------------------------------------------
// Machine
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
struct TransitionKey {
    from: State,
    event: String,
}

/// Immutable state machine definition.
///
/// # Example
/// ```rust
/// use tesela_runtime::workflow::{Machine, State, TransitionDef};
///
/// let draft = State::new("draft");
/// let review = State::new("review");
/// let merged = State::new("merged");
///
/// let m = Machine::new("branch", draft.clone(), vec![
///     TransitionDef { from: draft.clone(), to: review.clone(), event: "submit".into(), guard: None },
///     TransitionDef { from: review.clone(), to: merged.clone(), event: "approve".into(), guard: None },
///     TransitionDef { from: review.clone(), to: draft.clone(), event: "revise".into(), guard: None },
/// ]);
///
/// let new_state = m.transition(&draft, "submit", &Default::default()).unwrap();
/// assert_eq!(new_state, review);
/// ```
pub struct Machine {
    name: String,
    initial: State,
    transitions: Vec<TransitionDef>,
    index: HashMap<TransitionKey, TransitionDef>,
}

impl Machine {
    /// Construct a new machine.
    pub fn new(name: impl Into<String>, initial: State, transitions: Vec<TransitionDef>) -> Self {
        let index = transitions
            .iter()
            .map(|t| {
                (
                    TransitionKey {
                        from: t.from.clone(),
                        event: t.event.clone(),
                    },
                    t.clone(),
                )
            })
            .collect();
        Self {
            name: name.into(),
            initial,
            transitions,
            index,
        }
    }

    /// Machine name.
    pub fn name(&self) -> &str {
        &self.name
    }

    /// Initial state.
    pub fn initial(&self) -> &State {
        &self.initial
    }

    /// All defined states (unique, in definition order).
    pub fn states(&self) -> Vec<State> {
        let mut seen = std::collections::HashSet::new();
        let mut out = Vec::new();
        for t in &self.transitions {
            if seen.insert(t.from.clone()) {
                out.push(t.from.clone());
            }
            if seen.insert(t.to.clone()) {
                out.push(t.to.clone());
            }
        }
        out
    }

    /// All transitions reachable from `from`.
    pub fn valid_transitions(&self, from: &State) -> Vec<&TransitionDef> {
        self.transitions
            .iter()
            .filter(|t| &t.from == from)
            .collect()
    }

    /// Return `true` if the `event` is defined from `from`.
    pub fn can_transition(&self, from: &State, event: &str) -> bool {
        self.index.contains_key(&TransitionKey {
            from: from.clone(),
            event: event.to_string(),
        })
    }

    /// Apply `event` from `from`, running the guard if present.
    ///
    /// Returns the new state on success, or an error if the transition is
    /// undefined or the guard rejects it.
    pub fn transition(
        &self,
        from: &State,
        event: &str,
        data: &std::collections::BTreeMap<String, tesela_core::Value>,
    ) -> Result<State, Error> {
        let t = self
            .index
            .get(&TransitionKey {
                from: from.clone(),
                event: event.to_string(),
            })
            .ok_or_else(|| {
                Error::validation(format!(
                    "workflow {}: no transition from {:?} on event {:?}",
                    self.name, from, event
                ))
            })?;

        if let Some(guard) = t.guard {
            guard(data).map_err(|e| {
                Error::validation(format!("workflow {}: guard failed: {}", self.name, e))
            })?;
        }

        Ok(t.to.clone())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::BTreeMap;

    fn branch_machine() -> Machine {
        let draft = State::new("draft");
        let review = State::new("review");
        let merged = State::new("merged");
        let discarded = State::new("discarded");
        Machine::new(
            "branch",
            draft.clone(),
            vec![
                TransitionDef {
                    from: draft.clone(),
                    to: review.clone(),
                    event: "submit".into(),
                    guard: None,
                },
                TransitionDef {
                    from: review.clone(),
                    to: merged.clone(),
                    event: "approve".into(),
                    guard: None,
                },
                TransitionDef {
                    from: review.clone(),
                    to: draft.clone(),
                    event: "revise".into(),
                    guard: None,
                },
                TransitionDef {
                    from: draft.clone(),
                    to: discarded.clone(),
                    event: "discard".into(),
                    guard: None,
                },
                TransitionDef {
                    from: review.clone(),
                    to: discarded.clone(),
                    event: "discard".into(),
                    guard: None,
                },
            ],
        )
    }

    #[test]
    fn valid_transition() {
        let m = branch_machine();
        let s = m
            .transition(&State::new("draft"), "submit", &BTreeMap::new())
            .unwrap();
        assert_eq!(s, State::new("review"));
    }

    #[test]
    fn undefined_transition_errors() {
        let m = branch_machine();
        assert!(
            m.transition(&State::new("merged"), "submit", &BTreeMap::new())
                .is_err()
        );
    }

    #[test]
    fn guard_can_block() {
        fn deny(_: &BTreeMap<String, tesela_core::Value>) -> Result<(), Error> {
            Err(Error::validation("blocked by guard"))
        }
        let m = Machine::new(
            "test",
            State::new("a"),
            vec![TransitionDef {
                from: State::new("a"),
                to: State::new("b"),
                event: "go".into(),
                guard: Some(deny),
            }],
        );
        assert!(
            m.transition(&State::new("a"), "go", &BTreeMap::new())
                .is_err()
        );
    }

    #[test]
    fn can_transition_check() {
        let m = branch_machine();
        assert!(m.can_transition(&State::new("draft"), "submit"));
        assert!(!m.can_transition(&State::new("draft"), "approve"));
    }
}
