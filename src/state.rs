use std::ops::Not;

use crate::backend::ShikaneBackend;
use crate::config::Profile;
use crate::config::ShikaneConfig;

use calloop::LoopSignal;
#[allow(unused_imports)]
use log::{debug, error, info, trace, warn};
use wayland_protocols_wlr::output_management::v1::client::zwlr_output_configuration_v1::ZwlrOutputConfigurationV1;

#[derive(Debug)]
pub(crate) struct ShikaneState {
    pub(crate) backend: ShikaneBackend,
    pub(crate) config: ShikaneConfig,
    loop_signal: LoopSignal,
    state: State,
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
    ShuttingDown,
}

#[derive(Clone, Copy, Debug)]
pub(crate) enum StateInput {
    OutputManagerDone,
    OutputManagerFinished,
    OutputConfigurationSucceeded,
}

impl ShikaneState {
    pub(crate) fn new(
        backend: ShikaneBackend,
        config: ShikaneConfig,
        loop_signal: LoopSignal,
    ) -> Self {
        Self {
            backend,
            config,
            loop_signal,
            state: State::StartingUp,
            output_config: None,
            applied_profile: None,
            selected_profile: None,
        }
    }

    fn select_profile(&mut self) {
        self.selected_profile = self
            .config
            .profiles
            .iter()
            .find(|profile| self.match_profile(profile))
            .cloned();

        match &self.selected_profile {
            Some(profile) => trace!("Selected profile: {}", profile.name),
            None => {
                warn!("No profile selected")
            }
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

        let mut unmatched_heads: Vec<&_> = self.backend.output_heads.values().into_iter().collect();
        // This code removes an `OutputHead` from `unmatched_heads` if it matches against the pattern in `output.r#match`
        profile.outputs.iter().for_each(|output| {
            unmatched_heads = unmatched_heads
                .iter()
                .filter_map(|head| head.matches(&output.r#match).not().then_some(*head))
                .collect();
        });
        // If the list of unmatched heads is empty then all heads have been matched against the provided outputs
        unmatched_heads.is_empty()
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
        };

        match (self.state, input) {
            (State::StartingUp, StateInput::OutputManagerDone) => {
                // OutputManager sent all information about current configuration
                self.select_profile();
                self.configure_selected_profile();
                self.output_config
                    .as_ref()
                    .expect("No profile configured")
                    .test();

                State::TestingProfile
            }
            (State::StartingUp, StateInput::OutputConfigurationSucceeded) => todo!(),
            (State::TestingProfile, StateInput::OutputManagerDone) => todo!(),
            (State::TestingProfile, StateInput::OutputConfigurationSucceeded) => {
                // Profile passed testing
                self.configure_selected_profile();
                self.output_config
                    .as_ref()
                    .expect("No profile configured")
                    .apply();

                State::ApplyingProfile
            }
            (State::ApplyingProfile, StateInput::OutputManagerDone) => {
                // OutputManager applied atomic changes to outputs
                // Do nothing
                State::ApplyingProfile
            }
            (State::ApplyingProfile, StateInput::OutputConfigurationSucceeded) => {
                // Profile is applied
                self.applied_profile = self.selected_profile.clone();
                self.backend.clean_up();

                State::ShuttingDown
            }
            (State::ProfileApplied, StateInput::OutputManagerDone) => todo!(),
            (State::ProfileApplied, StateInput::OutputConfigurationSucceeded) => todo!(),
            (State::ShuttingDown, StateInput::OutputManagerDone) => unreachable!(),
            (State::ShuttingDown, StateInput::OutputConfigurationSucceeded) => unreachable!(),
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
        }
    }
}
