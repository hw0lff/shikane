use std::collections::VecDeque;
use std::fmt::Display;

#[allow(unused_imports)]
use log::{debug, error, info, trace, warn};

use crate::error;
use crate::execute::CommandBuilder;
use crate::settings::Settings;
use crate::variant::{VSMInput, ValidVariant, VariantState};
use crate::wl_backend::{WlBackend, WlBackendEvent};

use super::profile_manager::ProfileManager;

pub struct DaemonStateMachine<B: WlBackend> {
    state: DSMState,
    skip_tests: bool,
    pub(crate) pm: ProfileManager,
    pub(crate) settings: Settings,
    pub(crate) backend: B,
    // true if the state machine encountered a shutdown
    encountered_shutdown: bool,
}

#[derive(Clone, Debug, Default)]
pub enum DSMState {
    #[default]
    NoVariantApplied,
    VariantInProgress(ValidVariant),
    VariantApplied(ValidVariant),
    RestartAfterResponse,
}

#[derive(Clone, Copy, Debug)]
pub enum DSMAction {
    Restart,
    TestVariant,
    ApplyVariant,
    TryNextVariant,
    ExecCmd,
    Inert,
}

impl<B: WlBackend> DaemonStateMachine<B> {
    pub fn new(backend: B, settings: Settings) -> Self {
        Self {
            state: Default::default(),
            skip_tests: settings.skip_tests,
            pm: ProfileManager::new(settings.profiles.clone()),
            settings,
            backend,
            encountered_shutdown: false,
        }
    }

    /// The returned bool tells the caller if the state machine has been shutdown.
    /// It cannot and will not keep running once it returns true.
    #[must_use]
    pub fn process_event_queue(&mut self, eq: VecDeque<WlBackendEvent>) -> bool {
        if self.has_shutdown() {
            return self.has_shutdown();
        }

        if eq.contains(&WlBackendEvent::NeededResourceFinished) {
            warn!("A needed resource finished. Shutting down.");
            self.state = self.shutdown();
            return self.has_shutdown();
        }

        for event in eq {
            self.advance(event);
            if self.has_shutdown() {
                return self.has_shutdown();
            }
        }

        // should be false at this point
        self.has_shutdown()
    }
    pub fn advance(&mut self, event: WlBackendEvent) {
        debug!("Advancing daemon state with event: {event}");
        self.state = self.next(event);
        info!("New daemon state: {}", self.state);
    }

    #[must_use]
    fn next(&mut self, event: WlBackendEvent) -> DSMState {
        use DSMState::*;
        use WlBackendEvent::*;
        match (self.state.clone(), event) {
            (NoVariantApplied, AtomicChangeDone) => self.restart(),
            (NoVariantApplied, NeededResourceFinished) => self.shutdown(),
            (NoVariantApplied, Succeeded | Failed | Cancelled) => self.warn_invalid(event),
            (VariantInProgress(v), event) => self.advance_variant(v, event),
            (VariantApplied(_), AtomicChangeDone) => self.restart(),
            (VariantApplied(_), NeededResourceFinished) => self.shutdown(),
            (VariantApplied(_), Succeeded | Failed | Cancelled) => self.warn_invalid(event),
            (RestartAfterResponse, AtomicChangeDone) => RestartAfterResponse,
            (RestartAfterResponse, NeededResourceFinished) => self.shutdown(),
            (RestartAfterResponse, Succeeded | Failed | Cancelled) => self.restart(),
        }
    }

    // This function is only called at [`DSMState::VariantInProgress`].
    fn advance_variant(&mut self, mut v: ValidVariant, event: WlBackendEvent) -> DSMState {
        let input = match event {
            WlBackendEvent::AtomicChangeDone => VSMInput::AtomicChangeDone,
            WlBackendEvent::NeededResourceFinished => {
                v.discard();
                return self.shutdown();
            }
            WlBackendEvent::Succeeded => VSMInput::Succeeded,
            WlBackendEvent::Failed => VSMInput::Failed,
            WlBackendEvent::Cancelled => VSMInput::Cancelled,
        };

        let action = v.state.advance(input);
        // Set the current variant and variant state as the variant in progress.
        // Do the action only with the advanced variant as they rely on the current DSM state!
        self.state = DSMState::VariantInProgress(v.clone());
        self.do_action(action, v)
    }

    pub fn do_action(&mut self, action: DSMAction, variant: ValidVariant) -> DSMState {
        match action {
            DSMAction::Restart => self.restart(),
            DSMAction::TestVariant => {
                if let Err(err) = self.backend.test(&variant) {
                    warn!("{}", error::report(&err));
                }
                DSMState::VariantInProgress(variant)
            }
            DSMAction::ApplyVariant => {
                if let Err(err) = self.backend.apply(&variant) {
                    warn!("{}", error::report(&err));
                }
                DSMState::VariantInProgress(variant)
            }
            DSMAction::TryNextVariant => self.next_variant(),
            DSMAction::ExecCmd => {
                self.execute_variant_commands(&variant);
                if self.settings.oneshot {
                    // No return here because the variant is applied.
                    self.shutdown();
                }
                DSMState::VariantApplied(variant)
            }
            DSMAction::Inert => self.state.clone(),
        }
    }

    pub fn next_variant(&mut self) -> DSMState {
        match self.pm.next_variant() {
            None => DSMState::NoVariantApplied,
            Some(mut variant) => {
                let action = variant.start(self.skip_tests);
                self.do_action(action, variant)
            }
        }
    }

    pub fn restart(&mut self) -> DSMState {
        if let DSMState::VariantInProgress(v) = &self.state {
            if let VariantState::Testing | VariantState::Applying = v.state {
                return DSMState::RestartAfterResponse;
            }
        }

        if let Some(heads) = self.backend.export_heads() {
            // If there is no relevant change, don't restart.
            if !self.pm.is_cache_outdated(&heads) {
                debug!("No relevant change in heads detected. Not restarting.");
                return self.state.clone();
            }
            // Else regenerate variants.
            self.pm.clear();
            self.pm.generate_variants(heads);
        }
        self.next_variant()
    }

    pub fn shutdown(&mut self) -> DSMState {
        self.encountered_shutdown = true;
        DSMState::NoVariantApplied
    }

    pub fn has_shutdown(&self) -> bool {
        self.encountered_shutdown
    }

    pub fn state(&self) -> &DSMState {
        &self.state
    }

    pub fn simulate_change(&mut self) {
        debug!("simulating change");
        self.pm.clear_cached_heads();
        self.advance(WlBackendEvent::AtomicChangeDone);
    }

    fn execute_variant_commands(&self, variant: &ValidVariant) {
        let mut cmdb = CommandBuilder::new(variant.profile.name.clone());
        cmdb.oneshot(self.settings.oneshot);
        if let Some(profile_commands) = &variant.profile.commands {
            cmdb.profile_commands(profile_commands.clone())
        }
        for pairing in variant.pairings.iter() {
            if let Some(output_commands) = &pairing.output().commands {
                let head_name = pairing.wl_head().name().to_owned();
                cmdb.insert_head_commands(head_name, output_commands.clone());
            }
        }

        cmdb.execute();
    }

    /// Prints a warning and does not advance itself, returns its current state.
    #[must_use]
    fn warn_invalid(&self, event: WlBackendEvent) -> DSMState {
        warn!("Received invalid input {event} at state {}", self.state);
        self.state.clone()
    }
}

impl Display for DSMState {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            DSMState::NoVariantApplied => write!(f, "NoVariantApplied"),
            DSMState::VariantInProgress(v) => {
                write!(f, "VariantInProgress {}:{:?}", v.idx_str(), v.profile.name)
            }
            DSMState::VariantApplied(v) => {
                write!(f, "VariantApplied {}:{:?}", v.idx_str(), v.profile.name)
            }
            DSMState::RestartAfterResponse => write!(f, "RestartAfterResponse"),
        }
    }
}
