use std::fmt::Display;

#[allow(unused_imports)]
use log::{debug, error, info, trace, warn};
use serde::{Deserialize, Serialize};

use crate::daemon::state_machine::DSMAction;
use crate::matching::Pairing;
use crate::profile::Profile;

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ValidVariant {
    pub profile: Profile,
    pub pairings: Vec<Pairing>,
    pub state: VariantState,
    pub index: usize,
}

#[derive(Copy, Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum VariantState {
    /// Initial state
    #[default]
    Untested,
    Testing,
    Applying,
    /// Final state
    Applied,
    /// Final state
    Discarded,
}

/// variant state machine input
#[derive(Copy, Clone, Debug)]
pub enum VSMInput {
    Succeeded,
    Cancelled,
    Failed,
    AtomicChangeDone,
}

impl ValidVariant {
    #[must_use]
    pub fn start(&mut self, skip_tests: bool) -> DSMAction {
        self.state.reset();
        if skip_tests {
            debug!("skipping test by simulating a successful test");
            self.state = VariantState::Applying;
            info!("Initial variant state: {} (test skipped)", self.state);
            return DSMAction::ApplyVariant;
        }
        info!("Initial variant state: {}", self.state);
        self.state.advance(VSMInput::AtomicChangeDone)
    }

    pub fn discard(&mut self) {
        self.state = VariantState::Discarded;
    }

    pub fn mode_deviation(&self) -> u32 {
        self.pairings.iter().map(|p| p.mode_deviation()).sum()
    }

    pub fn specificity(&self) -> u64 {
        self.pairings.iter().map(|p| p.specificity()).sum::<u64>() / self.pairings.len() as u64
    }
    pub fn idx_str(&self) -> String {
        format!("{},{}", self.profile.index, self.index)
    }
}

impl VariantState {
    /// Advance itself by the given input, returning a [`DSMAction`]
    #[must_use]
    pub fn advance(&mut self, input: VSMInput) -> DSMAction {
        debug!("Advancing variant state with input: {input}");
        let (new_state, action) = self.next(input);
        *self = new_state;
        info!("New variant state: {}", self);
        action
    }

    /// Consumes itself and an input, returns a new instance of itself and a [`DSMAction`].
    #[must_use]
    pub fn next(self, input: VSMInput) -> (Self, DSMAction) {
        use DSMAction::*;
        use VSMInput::*;
        use VariantState::*;
        match (self, input) {
            (Untested, AtomicChangeDone) => (Testing, TestVariant),
            (Untested, _) => self.warn_invalid(input),
            (Testing, Succeeded) => (Applying, ApplyVariant),
            (Testing, Cancelled) => (Discarded, Restart),
            (Testing, Failed) => (Discarded, TryNextVariant),
            (Testing, AtomicChangeDone) => (self, Inert),
            (Applying, Succeeded) => (Applied, ExecCmd),
            (Applying, Cancelled) => (Discarded, Restart),
            (Applying, Failed) => (Discarded, TryNextVariant),
            (Applying, AtomicChangeDone) => (self, Inert),
            (Applied, AtomicChangeDone) => (Discarded, Restart),
            (Applied, _) => self.warn_invalid(input),
            (Discarded, _) => self.warn_invalid(input),
        }
    }

    /// Prints a warning and does not advance itself, returns itself and [`DSMAction::Inert`].
    #[must_use]
    fn warn_invalid(self, input: VSMInput) -> (Self, DSMAction) {
        warn!("Received invalid input {input} at state {self}");
        (self, DSMAction::Inert)
    }

    pub fn reset(&mut self) {
        *self = Default::default()
    }
}

impl Display for VSMInput {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let text = match self {
            VSMInput::Succeeded => "Succeeded",
            VSMInput::Cancelled => "Cancelled",
            VSMInput::Failed => "Failed",
            VSMInput::AtomicChangeDone => "AtomicChangeDone",
        };
        write!(f, "{text}")
    }
}

impl Display for VariantState {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let text = match self {
            VariantState::Untested => "Untested",
            VariantState::Testing => "Testing",
            VariantState::Applying => "Applying",
            VariantState::Applied => "Applied",
            VariantState::Discarded => "Discarded",
        };
        write!(f, "{text}")
    }
}
