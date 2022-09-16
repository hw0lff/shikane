use crate::args::ShikaneArgs;
use crate::backend::ShikaneBackend;
use crate::config::Profile;
use crate::config::ShikaneConfig;

use calloop::LoopSignal;
#[allow(unused_imports)]
use log::{debug, error, info, trace, warn};
use wayland_protocols_wlr::output_management::v1::client::zwlr_output_configuration_v1::ZwlrOutputConfigurationV1;

#[derive(Debug)]
pub(crate) struct ShikaneState {
    pub(crate) args: ShikaneArgs,
    pub(crate) backend: ShikaneBackend,
    pub(crate) config: ShikaneConfig,
    loop_signal: LoopSignal,
    state: State,
    list_of_unchecked_profiles: Vec<Profile>,
    output_config: Option<ZwlrOutputConfigurationV1>,
    applied_profile: Option<Profile>,
    selected_profile: Option<Profile>,
}

#[derive(Clone, Copy, Debug)]
enum State {
    StartingUp,
    TestingProfile,
    ApplyingProfile,
    ProfileApplied,
    NoProfileApplied,
    ShuttingDown,
}

#[derive(Clone, Copy, Debug)]
pub(crate) enum StateInput {
    OutputManagerDone,
    OutputManagerFinished,
    OutputConfigurationSucceeded,
    OutputConfigurationFailed,
    OutputConfigurationCancelled,
}

impl ShikaneState {
    pub(crate) fn new(
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

    fn configure_selected_profile(&mut self) {
        if let Some(profile) = &self.selected_profile {
            let output_config = self.backend.create_configuration();
            debug!("Configuring profile: {}", profile.name);

            profile.outputs.iter().for_each(|output| {
                let (head_id, output_head) = self.backend.match_head(&output.r#match).unwrap();
                trace!("Setting Head: {:?}", output_head.name);
                let head = self.backend.head_from_id(head_id.clone());

                if output.enable {
                    let opch =
                        output_config.enable_head(&head, &self.backend.qh, self.backend.data);
                    // Mode
                    let (mode_id, output_mode) =
                        self.backend.match_mode(head_id, &output.mode).unwrap();
                    trace!("Setting Mode: {:?}", output_mode);
                    let mode = self.backend.mode_from_id(mode_id);
                    opch.set_mode(&mode);

                    // Position
                    trace!("Setting position: {:?}", output.position);
                    opch.set_position(output.position.x, output.position.y);
                } else {
                    output_config.disable_head(&head);
                }
            });

            self.output_config = Some(output_config);
        }
    }

    fn select_next_profile_then_configure_and_test(&mut self) -> State {
        self.selected_profile = self.list_of_unchecked_profiles.pop();
        match &self.selected_profile {
            Some(profile) => trace!("Selected profile: {}", profile.name),
            None => {
                warn!("No profiles matched the currently connected outputs");
                if self.args.oneshot {
                    self.backend.clean_up();
                    return State::ShuttingDown;
                }
                return State::NoProfileApplied;
            }
        }
        self.configure_selected_profile();
        if self.args.skip_tests {
            return self.apply_configured_profile();
        }
        self.output_config
            .as_ref()
            .expect("No profile configured")
            .test();

        State::TestingProfile
    }

    fn apply_configured_profile(&mut self) -> State {
        self.output_config
            .as_ref()
            .expect("No profile configured")
            .apply();

        State::ApplyingProfile
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

    pub(crate) fn idle(&mut self) {
        self.backend.flush();
    }

    pub(crate) fn advance(&mut self, input: StateInput) {
        trace!("Previous state: {:?}, input: {:?}", self.state, input);
        let next_state = self.match_input(input);
        trace!("Next state: {:?}", next_state);
        self.state = next_state;
    }

    fn match_input(&mut self, input: StateInput) -> State {
        match input {
            StateInput::OutputManagerDone => {}
            StateInput::OutputManagerFinished => {}
            StateInput::OutputConfigurationSucceeded => self.destroy_config(),
            StateInput::OutputConfigurationFailed => self.destroy_config(),
            StateInput::OutputConfigurationCancelled => self.destroy_config(),
        };

        match (self.state, input) {
            (State::StartingUp, StateInput::OutputManagerDone) => {
                // OutputManager sent all information about current configuration
                self.create_list_of_unchecked_profiles();
                self.select_next_profile_then_configure_and_test()
            }
            (State::TestingProfile, StateInput::OutputManagerDone) => {
                // OutputManager applied atomic changes to outputs.
                // If outdated information has been sent to the server
                // we will get the Cancelled event.
                //
                // Do nothing
                State::TestingProfile
            }
            (State::TestingProfile, StateInput::OutputConfigurationSucceeded) => {
                // Profile passed testing
                self.configure_selected_profile();
                self.apply_configured_profile()
            }
            (State::TestingProfile, StateInput::OutputConfigurationFailed) => {
                self.select_next_profile_then_configure_and_test()
            }
            (State::TestingProfile, StateInput::OutputConfigurationCancelled) => {
                self.create_list_of_unchecked_profiles();
                self.select_next_profile_then_configure_and_test()
            }
            (State::ApplyingProfile, StateInput::OutputManagerDone) => {
                // OutputManager applied atomic changes to outputs.
                // If outdated information has been sent to the server
                // we will get the Cancelled event.
                //
                // Do nothing
                State::ApplyingProfile
            }
            (State::ApplyingProfile, StateInput::OutputConfigurationSucceeded) => {
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
                                            .arg(&cmd)
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

                if self.args.oneshot {
                    self.backend.clean_up();
                    return State::ShuttingDown;
                }

                State::ProfileApplied
            }
            (State::ApplyingProfile, StateInput::OutputConfigurationFailed) => {
                self.select_next_profile_then_configure_and_test()
            }
            (State::ApplyingProfile, StateInput::OutputConfigurationCancelled) => {
                self.create_list_of_unchecked_profiles();
                self.select_next_profile_then_configure_and_test()
            }
            (State::ProfileApplied, StateInput::OutputManagerDone) => {
                // OutputManager sent new information about current configuration
                self.create_list_of_unchecked_profiles();
                self.select_next_profile_then_configure_and_test()
            }
            (State::NoProfileApplied, StateInput::OutputManagerDone) => {
                // OutputManager sent new information about current configuration
                self.create_list_of_unchecked_profiles();
                self.select_next_profile_then_configure_and_test()
            }
            (State::ShuttingDown, StateInput::OutputManagerDone) => unreachable!(),
            (State::ShuttingDown, StateInput::OutputManagerFinished) => {
                trace!("Stopping event loop");
                self.loop_signal.stop();
                State::ShuttingDown
            }
            (_, StateInput::OutputManagerFinished) => {
                error!(
                    "OutputManager has finished unexpectedly. State: {:?}",
                    self.state
                );
                trace!("Stopping event loop");
                self.loop_signal.stop();
                State::ShuttingDown
            }
            (_, StateInput::OutputConfigurationSucceeded) => unreachable!(),
            (_, StateInput::OutputConfigurationFailed) => unreachable!(),
            (_, StateInput::OutputConfigurationCancelled) => unreachable!(),
        }
    }
}
