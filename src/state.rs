use std::collections::VecDeque;
use std::fmt::Display;

use crate::backend::ShikaneBackend;
use crate::config::ShikaneConfig;
use crate::daemon::ShikaneDaemonArgs;
use crate::error::ShikaneError;
use crate::exec::execute_plan_commands;
use crate::profile;
use crate::profile::ShikaneProfilePlan;

use calloop::LoopSignal;
#[allow(unused_imports)]
use log::{debug, error, info, trace, warn};

#[derive(Debug)]
pub struct ShikaneState {
    pub args: ShikaneDaemonArgs,
    pub backend: ShikaneBackend,
    pub config: ShikaneConfig,
    loop_signal: LoopSignal,
    pub(crate) state: State,
    unchecked_plans: VecDeque<ShikaneProfilePlan>,
}

#[derive(Clone, Debug)]
pub(crate) enum State {
    StartingUp,
    TestingProfile(ShikaneProfilePlan),
    ApplyingProfile(ShikaneProfilePlan),
    ProfileApplied(ShikaneProfilePlan),
    NoProfileApplied,
    ShuttingDown,
}

#[allow(clippy::enum_variant_names)]
#[derive(Clone, Copy, Debug)]
pub enum StateInput {
    OutputManagerDone,
    OutputManagerFinished,
    OutputConfigurationSucceeded,
    OutputConfigurationFailed,
    OutputConfigurationCancelled,
}

impl ShikaneState {
    pub fn new(
        args: ShikaneDaemonArgs,
        backend: ShikaneBackend,
        config: ShikaneConfig,
        loop_signal: LoopSignal,
    ) -> Self {
        Self {
            args,
            backend,
            config,
            loop_signal,
            state: State::StartingUp,
            unchecked_plans: VecDeque::new(),
        }
    }

    fn configure_next_plan(&mut self) -> Result<State, ShikaneError> {
        let plan = match self.unchecked_plans.pop_front() {
            Some(plan) => {
                trace!("Selected profile: {}", plan.profile.name);
                plan
            }
            None => {
                warn!("No profiles matched the currently connected outputs");
                if self.args.oneshot {
                    self.backend.clean_up();
                    return Ok(State::ShuttingDown);
                }
                return Ok(State::NoProfileApplied);
            }
        };

        if self.args.skip_tests {
            self.apply_plan(plan)
        } else {
            self.test_plan(plan)
        }
    }

    fn test_plan(&mut self, plan: ShikaneProfilePlan) -> Result<State, ShikaneError> {
        let configuration = plan.configure(&mut self.backend)?;
        configuration.test();
        Ok(State::TestingProfile(plan))
    }

    fn apply_plan(&mut self, plan: ShikaneProfilePlan) -> Result<State, ShikaneError> {
        let configuration = plan.configure(&mut self.backend)?;
        configuration.apply();
        Ok(State::ApplyingProfile(plan))
    }

    fn create_list_of_unchecked_plans(&mut self) {
        self.unchecked_plans = profile::create_profile_plans(&self.config.profiles, &self.backend)
    }

    pub fn idle(&mut self) -> Result<(), ShikaneError> {
        self.backend.flush()
    }

    pub fn advance(&mut self, input: StateInput) {
        debug!("Previous state: {}, input: {}", self.state, input);
        let next_state = match self.match_input(input) {
            Ok(s) => s,
            Err(err @ ShikaneError::Configuration(_)) => {
                warn!("{}, Restarting", err);
                State::StartingUp
            }
            Err(err) => {
                error!("{}", err);
                self.backend.clean_up();
                State::ShuttingDown
            }
        };
        debug!("Next state: {}", next_state);
        self.state = next_state;
    }

    fn match_input(&mut self, input: StateInput) -> Result<State, ShikaneError> {
        match (self.state.clone(), input) {
            (State::StartingUp, StateInput::OutputManagerDone) => {
                // OutputManager sent all information about current configuration
                self.create_list_of_unchecked_plans();
                self.configure_next_plan()
            }
            (State::TestingProfile(plan), StateInput::OutputConfigurationSucceeded) => {
                // Profile passed testing
                self.apply_plan(plan)
            }
            (State::ApplyingProfile(plan), StateInput::OutputConfigurationSucceeded) => {
                // Profile is applied
                execute_plan_commands(&plan, self.args.oneshot);
                info!("Profile applied: {}", plan.profile.name);

                if self.args.oneshot {
                    self.backend.clean_up();
                    return Ok(State::ShuttingDown);
                }

                Ok(State::ProfileApplied(plan))
            }
            (
                State::TestingProfile(_) | State::ApplyingProfile(_),
                StateInput::OutputConfigurationFailed,
            ) => {
                // Failed means that this profile (configuration) cannot work
                self.configure_next_plan()
            }
            (State::TestingProfile(plan), StateInput::OutputConfigurationCancelled) => {
                // Cancelled means that we have outdated information
                self.create_list_of_unchecked_plans();
                // If the newly selected plan is the same as the one that is currently being tested
                // then try testing the same plan again
                if let Some(selected_plan) = self.unchecked_plans.front() {
                    if *selected_plan == plan {
                        return self.test_plan(plan);
                    }
                }
                // Else configure the next plan
                self.configure_next_plan()
            }
            (State::ApplyingProfile(plan), StateInput::OutputConfigurationCancelled) => {
                // Cancelled means that we have outdated information
                self.create_list_of_unchecked_plans();
                // If the newly selected plan is the same as the one that is currently being applied
                // then try applying the same plan again
                if let Some(selected_plan) = self.unchecked_plans.front() {
                    if *selected_plan == plan {
                        return self.apply_plan(plan);
                    }
                }
                // Else configure the next plan
                self.configure_next_plan()
            }
            (State::ProfileApplied(applied_plan), StateInput::OutputManagerDone) => {
                // OutputManager sent new information about current configuration
                self.create_list_of_unchecked_plans();
                // If the newly selected profile is the same as the one that is already applied
                // then do nothing
                if let Some(selected_plan) = self.unchecked_plans.front() {
                    if *selected_plan == applied_plan {
                        return Ok(State::ProfileApplied(applied_plan));
                    }
                }
                self.configure_next_plan()
            }
            (State::TestingProfile(plan), StateInput::OutputManagerDone) => {
                // OutputManager applied atomic changes to outputs.
                // If outdated information has been sent to the server
                // we will get the Cancelled event.
                //
                // Do nothing
                Ok(State::TestingProfile(plan))
            }
            (State::ApplyingProfile(plan), StateInput::OutputManagerDone) => {
                // OutputManager applied atomic changes to outputs.
                // If outdated information has been sent to the server
                // we will get the Cancelled event.
                //
                // Do nothing
                Ok(State::ApplyingProfile(plan))
            }
            (State::NoProfileApplied, StateInput::OutputManagerDone) => {
                // OutputManager sent new information about current configuration
                self.create_list_of_unchecked_plans();
                self.configure_next_plan()
            }
            (state @ State::ShuttingDown, input @ StateInput::OutputManagerDone) => {
                warn!("Reached unexpected state \"{state}\" with input \"{input}\". Continuing as if it had not occurred.");
                Ok(state)
            }
            (State::ShuttingDown, StateInput::OutputManagerFinished) => {
                trace!("Stopping event loop");
                self.loop_signal.stop();
                Ok(State::ShuttingDown)
            }
            (state, StateInput::OutputManagerFinished) => {
                error!("OutputManager has finished unexpectedly. State: {state}",);
                trace!("Stopping event loop");
                self.loop_signal.stop();
                Ok(State::ShuttingDown)
            }
            (
                state,
                input @ StateInput::OutputConfigurationSucceeded
                | input @ StateInput::OutputConfigurationFailed
                | input @ StateInput::OutputConfigurationCancelled,
            ) => {
                warn!("Reached unexpected state \"{state}\" with input \"{input}\". Continuing as if it had not occurred.");
                Ok(state)
            }
        }
    }
}

impl Display for State {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            State::StartingUp => write!(f, "StartingUp"),
            State::TestingProfile(_) => write!(f, "TestingProfile"),
            State::ApplyingProfile(_) => write!(f, "ApplyingProfile"),
            State::ProfileApplied(_) => write!(f, "ProfileApplied"),
            State::NoProfileApplied => write!(f, "NoProfileApplied"),
            State::ShuttingDown => write!(f, "ShuttingDown"),
        }
    }
}

impl Display for StateInput {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            StateInput::OutputManagerDone => write!(f, "OutputManagerDone"),
            StateInput::OutputManagerFinished => write!(f, "OutputManagerFinished"),
            StateInput::OutputConfigurationSucceeded => write!(f, "OutputConfigurationSucceeded"),
            StateInput::OutputConfigurationFailed => write!(f, "OutputConfigurationFailed"),
            StateInput::OutputConfigurationCancelled => write!(f, "OutputConfigurationCancelled"),
        }
    }
}
