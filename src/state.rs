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
    list_of_unchecked_profiles: Vec<Profile>,
    output_config: Option<ZwlrOutputConfigurationV1>,
    applied_profile: Option<Profile>,
    selected_profile: Option<Profile>,
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
            list_of_unchecked_profiles: Vec::new(),
            output_config: None,
            applied_profile: None,
            selected_profile: None,
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
        self.selected_profile = self.list_of_unchecked_profiles.pop();
        let profile = match &self.selected_profile {
            Some(profile) => {
                trace!("Selected profile: {}", profile.name);
                profile.clone()
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

        let configuration = self.configure_profile(&profile)?;
        if self.args.skip_tests {
            configuration.apply();
            self.output_config = Some(configuration);
            Ok(State::ApplyingProfile(profile))
        } else {
            configuration.test();
            self.output_config = Some(configuration);
            Ok(State::TestingProfile(profile))
        }
    }

    fn create_list_of_unchecked_profiles(&mut self) {
        self.list_of_unchecked_profiles = self
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
        trace!("Previous state: {:?}, input: {:?}", self.state, input);
        let next_state = match self.match_input(input) {
            Ok(s) => s,
            Err(err) => {
                error!("{}", err);
                self.backend.clean_up();
                State::ShuttingDown
            }
        };
        trace!("Next state: {:?}", next_state);
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
            (State::TestingProfile(profile), StateInput::OutputManagerDone) => {
                // OutputManager applied atomic changes to outputs.
                // If outdated information has been sent to the server
                // we will get the Cancelled event.
                //
                // Do nothing
                Ok(State::TestingProfile(profile))
            }
            (State::TestingProfile(profile), StateInput::OutputConfigurationSucceeded) => {
                // Profile passed testing
                let configuration = self.configure_profile(&profile)?;
                configuration.apply();
                self.output_config = Some(configuration);
                Ok(State::ApplyingProfile(profile))
            }
            (State::TestingProfile(profile), StateInput::OutputConfigurationFailed) => {
                // Failed means that this profile (configuration) cannot work
                self.configure_next_profile()
            }
            (State::TestingProfile(profile), StateInput::OutputConfigurationCancelled) => {
                // Cancelled means that we can try again
                let configuration = self.configure_profile(&profile)?;
                configuration.test();
                self.output_config = Some(configuration);
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
            (State::ApplyingProfile(profile), StateInput::OutputConfigurationSucceeded) => {
                // Profile is applied
                self.applied_profile = self.selected_profile.clone();
                if let Some(profile) = &self.applied_profile {
                    if let Some(exec) = &profile.exec {
                        let exec = exec.clone();
                        trace!("Starting command exec thread");
                        let handle = std::thread::Builder::new()
                            .name("command exec".into())
                            .spawn(move || {
                                exec.iter().for_each(|cmd| {
                                    if !cmd.is_empty() {
                                        trace!("[Exec] {:?}", cmd);
                                        match std::process::Command::new("sh")
                                            .arg("-c")
                                            .arg(cmd)
                                            .output()
                                        {
                                            Ok(output) => {
                                                if let Ok(stdout) = String::from_utf8(output.stdout)
                                                {
                                                    trace!("[ExecOutput] {:?}", stdout)
                                                }
                                            }

                                            Err(_) => error!("failed to spawn command: {:?}", cmd),
                                        }
                                    }
                                });
                            })
                            .expect("cannot spawn thread");

                        if self.args.oneshot {
                            match handle.join() {
                                Ok(_) => {}
                                Err(err) => {
                                    error!("[Exec] cannot join thread {:?}", err);
                                }
                            };
                        }
                    }
                }

                if let Some(ref profile) = self.applied_profile {
                    info!("Profile applied: {}", profile.name);
                }

                if self.args.oneshot {
                    self.backend.clean_up();
                    return Ok(State::ShuttingDown);
                }

                Ok(State::ProfileApplied(profile))
            }
            (State::ApplyingProfile(profile), StateInput::OutputConfigurationFailed) => {
                // Failed means that this profile (configuration) cannot work
                self.configure_next_profile()
            }
            (State::ApplyingProfile(profile), StateInput::OutputConfigurationCancelled) => {
                // Cancelled means that we can try again
                let configuration = self.configure_profile(&profile)?;
                configuration.apply();
                self.output_config = Some(configuration);
                Ok(State::ApplyingProfile(profile))
            }
            (State::ProfileApplied(profile), StateInput::OutputManagerDone) => {
                // OutputManager sent new information about current configuration
                self.create_list_of_unchecked_profiles();
                self.configure_next_profile()
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
