use crate::backend::ShikaneBackend;
use crate::error::ShikaneError;

use serde::Deserialize;
use wayland_client::Proxy;
use wayland_protocols_wlr::output_management::v1::client::zwlr_output_configuration_v1::ZwlrOutputConfigurationV1;
use wayland_protocols_wlr::output_management::v1::client::zwlr_output_head_v1::ZwlrOutputHeadV1;
use wayland_protocols_wlr::output_management::v1::client::zwlr_output_mode_v1::ZwlrOutputModeV1;

#[allow(unused_imports)]
use log::{debug, error, info, trace, warn};

#[derive(Clone, Default, Debug, Deserialize, PartialEq, Eq)]
pub struct Position {
    pub x: i32,
    pub y: i32,
}
#[derive(Clone, Default, Debug, Deserialize, PartialEq, Eq)]
pub struct Mode {
    pub width: i32,
    pub height: i32,
    pub refresh: i32,
}
#[derive(Clone, Default, Debug, Deserialize, PartialEq, Eq)]
pub struct Output {
    pub enable: bool,
    pub r#match: String,
    pub mode: Mode,
    pub position: Position,
}
#[derive(Clone, Default, Debug, Deserialize, PartialEq, Eq)]
pub struct Profile {
    pub name: String,
    #[serde(rename = "output")]
    pub outputs: Vec<Output>,
    pub exec: Option<Vec<String>>,
}

pub fn match_profile(backend: &ShikaneBackend, profile: &Profile) -> bool {
    if profile.outputs.len() != backend.output_heads.len() {
        return false;
    }

    let mut matches: usize = 0;
    'output_loop: for output in profile.outputs.iter() {
        for head in backend.output_heads.values() {
            if head.matches(&output.r#match) {
                matches += 1;
                continue 'output_loop;
            }
        }
    }
    backend.output_heads.len() == matches
}

pub fn configure_profile(
    backend: &mut ShikaneBackend,
    profile: &Profile,
) -> Result<ZwlrOutputConfigurationV1, ShikaneError> {
    let output_config = backend.create_configuration();
    debug!("Configuring profile: {}", profile.name);

    for output in profile.outputs.iter() {
        let (head_id, output_head) = backend
            .match_head2(&output.r#match)
            .ok_or_else(|| ShikaneError::Configuration(profile.name.clone()))?;
        trace!("Setting Head: {:?}", output_head.name);
        let head = backend.head_from_id(head_id.clone())?;

        // disable the head if is disabled in the config
        if !output.enable {
            output_config.disable_head(&head);
            continue;
        }

        // enable the head and set its properties
        let opch = output_config.enable_head(&head, &backend.qh, backend.data);
        // Mode
        let (mode_id, output_mode) = backend
            .match_mode2(head_id, &output.mode)
            .ok_or_else(|| ShikaneError::Configuration(profile.name.clone()))?;
        trace!("Setting Mode: {:?}", output_mode);
        let mode = backend.mode_from_id(mode_id)?;
        opch.set_mode(&mode);

        // Position
        trace!("Setting position: {:?}", output.position);
        opch.set_position(output.position.x, output.position.y);
    }

    Ok(output_config)
}

#[derive(Clone, Debug)]
pub struct ShikaneProfilePlan {
    pub profile: Profile,
    config_set: Vec<(Output, ZwlrOutputHeadV1, ZwlrOutputModeV1)>,
}

impl ShikaneProfilePlan {
    pub fn configure(
        &self,
        backend: &mut ShikaneBackend,
    ) -> Result<ZwlrOutputConfigurationV1, ShikaneError> {
        let configuration = backend.create_configuration();
        debug!("Configuring profile: {}", self.profile.name);

        for (output, head, mode) in self.config_set.iter() {
            // Cannot configure a head or a mode that is not alive
            if !head.is_alive() || !mode.is_alive() {
                return Err(ShikaneError::Configuration(self.profile.name.clone()));
            }

            // Disable the head if is disabled in the config
            if !output.enable {
                configuration.disable_head(head);
                continue;
            }

            // Enable the head and set its properties
            let configuration_head = configuration.enable_head(head, &backend.qh, backend.data);

            // Mode
            trace!("Setting Mode: {:?}", output.mode);
            configuration_head.set_mode(mode);

            // Position
            trace!("Setting Position: {:?}", output.position);
            configuration_head.set_position(output.position.x, output.position.y);
        }

        Ok(configuration)
    }
}

pub fn create_profile_plans(
    profiles: &[Profile],
    backend: &ShikaneBackend,
) -> Vec<ShikaneProfilePlan> {
    trace!("[Create Profile Plans]");
    let mut profile_plans = vec![];
    for profile in profiles.iter() {
        if profile.outputs.len() != backend.output_heads.len() {
            continue;
        }

        let mut config_set = vec![];
        'outputs: for output in profile.outputs.iter() {
            'heads: for o_head in backend.match_heads(&output.r#match) {
                // If the head has already been added to the config_set then skip it and look at
                // the next one
                if config_set.iter().any(|(_, wh, _)| *wh == o_head.wlr_head) {
                    trace!("[Skip Head] {}", o_head.name);
                    continue 'heads;
                }

                if let Some(o_mode) = backend.match_mode(o_head, &output.mode) {
                    trace!(
                        "[Head Matched] match: {}, head.name: {}, mode: {}",
                        output.r#match,
                        o_head.name,
                        o_mode
                    );
                    config_set.push((
                        output.clone(),
                        o_head.wlr_head.clone(),
                        o_mode.wlr_mode.clone(),
                    ));
                    continue 'outputs;
                }
            }
        }

        if config_set.len() == profile.outputs.len() {
            profile_plans.push(ShikaneProfilePlan {
                profile: profile.clone(),
                config_set,
            });
        }
    }

    profile_plans
}
