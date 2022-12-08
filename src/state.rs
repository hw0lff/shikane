use std::fmt::Display;

use crate::args::ShikaneArgs;
use crate::backend::ShikaneBackend;
use crate::config::Profile;
use crate::config::ShikaneConfig;
use crate::error::ShikaneError;

use calloop::LoopSignal;
#[allow(unused_imports)]
use log::{debug, error, info, trace, warn};
use wayland_protocols_wlr::output_management::v1::client::zwlr_output_configuration_v1::ZwlrOutputConfigurationV1;

#[derive(Debug)]
pub struct ShikaneState {
    pub args: ShikaneArgs,
    pub backend: ShikaneBackend,
    pub config: ShikaneConfig,
    loop_signal: LoopSignal,
    state: State,
    unchecked_profiles: Vec<Profile>,
    output_config: Option<ZwlrOutputConfigurationV1>,
}

#[derive(Clone, Debug)]
enum State {
    StartingUp,
    TestingProfile(Profile),
    ApplyingProfile(Profile),
    ProfileApplied(Profile),
    NoProfileApplied,
    ShuttingDown,
}

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
        args: ShikaneArgs,
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
            unchecked_profiles: Vec::new(),
            output_config: None,
        }
    }

    fn destroy_config(&mut self) {
        if let Some(config) = &self.output_config {
            config.destroy();
            self.output_config = None;
        }
    }

    fn match_profile(&self, profile: &Profile) -> bool {
        if profile.outputs.len() != self.backend.output_heads.len() {
            return false;
        }

        let mut matches: usize = 0;
        'output_loop: for output in profile.outputs.iter() {
            for head in self.backend.output_heads.values() {
                if head.matches(&output.r#match) {
                    matches += 1;
                    continue 'output_loop;
                }
            }
        }
        self.backend.output_heads.len() == matches
    }

    fn configure_profile(
        &mut self,
        profile: &Profile,
    ) -> Result<ZwlrOutputConfigurationV1, ShikaneError> {
        let output_config = self.backend.create_configuration();
        self.output_config = Some(output_config.clone());
        debug!("Configuring profile: {}", profile.name);

        for output in profile.outputs.iter() {
            let (head_id, output_head) = self
                .backend
                .match_head(&output.r#match)
                .ok_or(ShikaneError::ConfigurationError)?;
            trace!("Setting Head: {:?}", output_head.name);
            let head = self.backend.head_from_id(head_id.clone())?;

            // disable the head if is disabled in the config
            if !output.enable {
                output_config.disable_head(&head);
                continue;
            }

            // enable the head and set its properties
            let opch = output_config.enable_head(&head, &self.backend.qh, self.backend.data);
            // Mode
            let (mode_id, output_mode) = self
                .backend
                .match_mode(head_id, &output.mode)
                .ok_or(ShikaneError::ConfigurationError)?;
            trace!("Setting Mode: {:?}", output_mode);
            let mode = self.backend.mode_from_id(mode_id)?;
            opch.set_mode(&mode);

            // Position
            trace!("Setting position: {:?}", output.position);
            opch.set_position(output.position.x, output.position.y);
        }

        Ok(output_config)
    }

    fn configure_next_profile(&mut self) -> Result<State, ShikaneError> {
        loop {
            let profile = match self.unchecked_profiles.pop() {
                Some(profile) => {
                    trace!("Selected profile: {}", profile.name);
                    profile
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

            let next_state = if self.args.skip_tests {
                self.apply_profile(profile)
            } else {
                self.test_profile(profile)
            };

            // Destroy the possibly remaining ZwlrOutputConfigurationV1 if an error has occurred
            if next_state.is_err() {
                self.destroy_config();
            }

            if let Err(err @ ShikaneError::ConfigurationError) = next_state {
                warn!("{}", err);
                continue;
            }
            return next_state;
        }
    }

    fn test_profile(&mut self, profile: Profile) -> Result<State, ShikaneError> {
        let configuration = self.configure_profile(&profile)?;
        configuration.test();
        Ok(State::TestingProfile(profile))
    }

    fn apply_profile(&mut self, profile: Profile) -> Result<State, ShikaneError> {
        let configuration = self.configure_profile(&profile)?;
        configuration.apply();
        Ok(State::ApplyingProfile(profile))
    }

    fn create_list_of_unchecked_profiles(&mut self) {
        self.unchecked_profiles = self
            .config
            .profiles
            .iter()
            .filter(|profile| self.match_profile(profile))
            .cloned()
            .collect()
    }

    pub fn idle(&mut self) -> Result<(), ShikaneError> {
        self.backend.flush()
    }

    pub fn advance(&mut self, input: StateInput) {
        debug!("Previous state: {}, input: {}", self.state, input);
        let next_state = match self.match_input(input) {
            Ok(s) => s,
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
        match input {
            StateInput::OutputManagerDone => {}
            StateInput::OutputManagerFinished => {}
            StateInput::OutputConfigurationSucceeded => self.destroy_config(),
            StateInput::OutputConfigurationFailed => self.destroy_config(),
            StateInput::OutputConfigurationCancelled => self.destroy_config(),
        };

        match (self.state.clone(), input) {
            (State::StartingUp, StateInput::OutputManagerDone) => {
                // OutputManager sent all information about current configuration
                self.create_list_of_unchecked_profiles();
                self.configure_next_profile()
            }
            (State::TestingProfile(profile), StateInput::OutputConfigurationSucceeded) => {
                // Profile passed testing
                self.apply_profile(profile)
            }
            (State::ApplyingProfile(profile), StateInput::OutputConfigurationSucceeded) => {
                // Profile is applied
                execute_profile_commands(&profile, self.args.oneshot);
                info!("Profile applied: {}", profile.name);

                if self.args.oneshot {
                    self.backend.clean_up();
                    return Ok(State::ShuttingDown);
                }

                Ok(State::ProfileApplied(profile))
            }
            (
                State::TestingProfile(_) | State::ApplyingProfile(_),
                StateInput::OutputConfigurationFailed,
            ) => {
                // Failed means that this profile (configuration) cannot work
                self.configure_next_profile()
            }
            (State::TestingProfile(profile), StateInput::OutputConfigurationCancelled) => {
                // Cancelled means that we can try again
                self.test_profile(profile)
            }
            (State::ApplyingProfile(profile), StateInput::OutputConfigurationCancelled) => {
                // Cancelled means that we can try again
                self.apply_profile(profile)
            }
            (State::ProfileApplied(applied_profile), StateInput::OutputManagerDone) => {
                // OutputManager sent new information about current configuration
                self.create_list_of_unchecked_profiles();
                // If the newly selected profile is the same as the one that is already applied
                // then do nothing
                if let Some(selected_profile) = self.unchecked_profiles.first() {
                    if *selected_profile == applied_profile {
                        return Ok(State::ProfileApplied(applied_profile));
                    }
                }
                self.configure_next_profile()
            }
            (State::TestingProfile(profile), StateInput::OutputManagerDone) => {
                // OutputManager applied atomic changes to outputs.
                // If outdated information has been sent to the server
                // we will get the Cancelled event.
                //
                // Do nothing
                Ok(State::TestingProfile(profile))
            }
            (State::ApplyingProfile(profile), StateInput::OutputManagerDone) => {
                // OutputManager applied atomic changes to outputs.
                // If outdated information has been sent to the server
                // we will get the Cancelled event.
                //
                // Do nothing
                Ok(State::ApplyingProfile(profile))
            }
            (State::NoProfileApplied, StateInput::OutputManagerDone) => {
                // OutputManager sent new information about current configuration
                self.create_list_of_unchecked_profiles();
                self.configure_next_profile()
            }
            (State::ShuttingDown, StateInput::OutputManagerDone) => unreachable!(),
            (State::ShuttingDown, StateInput::OutputManagerFinished) => {
                trace!("Stopping event loop");
                self.loop_signal.stop();
                Ok(State::ShuttingDown)
            }
            (_, StateInput::OutputManagerFinished) => {
                error!(
                    "OutputManager has finished unexpectedly. State: {:?}",
                    self.state
                );
                trace!("Stopping event loop");
                self.loop_signal.stop();
                Ok(State::ShuttingDown)
            }
            (_, StateInput::OutputConfigurationSucceeded) => unreachable!(),
            (_, StateInput::OutputConfigurationFailed) => unreachable!(),
            (_, StateInput::OutputConfigurationCancelled) => unreachable!(),
        }
    }
}

fn execute_profile_commands(profile: &Profile, oneshot: bool) {
    if let Some(exec) = &profile.exec {
        let exec = exec.clone();
        trace!("[Exec] Starting command exec thread");
        let handle = match std::thread::Builder::new()
            .name("command exec".into())
            .spawn(move || {
                exec.iter().for_each(|cmd| execute_command(cmd));
            }) {
            Ok(joinhandle) => Some(joinhandle),
            Err(err) => {
                error!("[Exec] cannot spawn thread {:?}", err);
                None
            }
        };

        if !oneshot {
            return;
        }
        if let Some(handle) = handle {
            match handle.join() {
                Ok(_) => {}
                Err(err) => {
                    error!("[Exec] cannot join thread {:?}", err);
                }
            };
        }
    }
}

fn execute_command(cmd: &str) {
    use std::process::Command;
    if cmd.is_empty() {
        return;
    }
    debug!("[Exec] {:?}", cmd);
    match Command::new("sh").arg("-c").arg(cmd).output() {
        Ok(output) => {
            if let Ok(stdout) = String::from_utf8(output.stdout) {
                trace!("[ExecOutput] {:?}", stdout)
            }
        }
        Err(_) => error!("[Exec] failed to spawn command: {:?}", cmd),
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
