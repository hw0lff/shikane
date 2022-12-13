use std::fmt::Display;

use crate::backend::output_head::OutputHead;
use crate::backend::output_mode::OutputMode;
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
#[derive(Clone, Default, Debug, Deserialize, PartialEq)]
pub struct Output {
    pub enable: bool,
    pub r#match: String,
    pub mode: Option<Mode>,
    pub position: Option<Position>,
    pub scale: Option<f64>,
}
#[derive(Clone, Default, Debug, Deserialize, PartialEq)]
pub struct Profile {
    pub name: String,
    #[serde(rename = "output")]
    pub outputs: Vec<Output>,
    pub exec: Option<Vec<String>>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct ShikaneProfilePlan {
    pub profile: Profile,
    config_set: Vec<(Output, ZwlrOutputHeadV1, Option<ZwlrOutputModeV1>)>,
}

impl ShikaneProfilePlan {
    pub fn configure(
        &self,
        backend: &mut ShikaneBackend,
    ) -> Result<ZwlrOutputConfigurationV1, ShikaneError> {
        let configuration = backend.create_configuration();
        debug!("Configuring profile: {}", self.profile.name);

        for (output, wlr_head, wlr_mode) in self.config_set.iter() {
            // Cannot configure a head that is not alive
            if !wlr_head.is_alive() {
                return Err(ShikaneError::Configuration(self.profile.name.clone()));
            }

            // Disable the head if is disabled in the config
            if !output.enable {
                configuration.disable_head(wlr_head);
                continue;
            }

            // Enable the head and set its properties
            let configuration_head = configuration.enable_head(wlr_head, &backend.qh, backend.data);

            // Mode
            if let Some(mode) = &output.mode {
                if let Some(wlr_mode) = wlr_mode {
                    // Cannot configure a mode that is not alive
                    if !wlr_mode.is_alive() {
                        return Err(ShikaneError::Configuration(self.profile.name.clone()));
                    }
                    trace!("Setting Mode: {}", mode);
                    configuration_head.set_mode(wlr_mode);
                }
            }

            // Position
            if let Some(pos) = &output.position {
                trace!("Setting Position: {}", pos);
                configuration_head.set_position(pos.x, pos.y);
            }

            // Scale
            if let Some(scale) = &output.scale {
                trace!("Setting Scale: {}", scale);
                configuration_head.set_scale(*scale);
            }
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

        trace!("[Profile] {}", profile.name);

        let mut config_set = vec![];
        'outputs: for output in profile.outputs.iter() {
            'heads: for o_head in backend.match_heads(output) {
                // If the head has already been added to the config_set then skip it and look at
                // the next one
                if config_set.iter().any(|(_, wh, _)| *wh == o_head.wlr_head) {
                    trace!("[Skip Head] {}", o_head.name);
                    continue 'heads;
                }

                let mut mode_trace = String::new();
                let mut wlr_mode: Option<ZwlrOutputModeV1> = None;
                if let Some(mode) = &output.mode {
                    if let Some(o_mode) = backend.match_mode(o_head, mode) {
                        mode_trace = format!(", mode {}", o_mode);
                        wlr_mode = Some(o_mode.wlr_mode.clone());
                    }
                }

                trace!(
                    "[Head Matched] match: {}, head.name: {}{mode_trace}",
                    output.r#match,
                    o_head.name,
                );
                config_set.push((output.clone(), o_head.wlr_head.clone(), wlr_mode));
                continue 'outputs;
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

impl Mode {
    pub fn matches(&self, o_mode: &OutputMode, delta: &mut i32) -> bool {
        const MAX_DELTA: i32 = 500; // maximum difference in mHz
        let refresh: i32 = self.refresh * 1000; // convert Hz to mHZ
        let diff: i32 = refresh.abs_diff(o_mode.refresh) as i32; // difference in mHz
        trace!(
            "refresh: {refresh}mHz, monitor.refresh {}mHz, diff: {diff}mHz",
            o_mode.refresh
        );

        if diff < MAX_DELTA && diff < *delta {
            *delta = diff;
            return true;
        }
        false
    }
}

impl Output {
    pub fn matches(&self, o_head: &OutputHead) -> bool {
        o_head.name == self.r#match || o_head.make == self.r#match || o_head.model == self.r#match
    }
}

impl Display for Position {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{},{}", self.x, self.y)
    }
}

impl Display for Mode {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}x{}@{}Hz", self.width, self.height, self.refresh)
    }
}
