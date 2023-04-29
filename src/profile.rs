use std::collections::VecDeque;
use std::fmt::Display;

use crate::backend::output_head::OutputHead;
use crate::backend::output_mode::OutputMode;
use crate::backend::ShikaneBackend;
use crate::error::ShikaneError;
use crate::hk::HKMap;

use serde::Deserialize;
use wayland_client::protocol::wl_output::Transform;
use wayland_client::Proxy;
use wayland_protocols_wlr::output_management::v1::client::zwlr_output_configuration_v1::ZwlrOutputConfigurationV1;
use wayland_protocols_wlr::output_management::v1::client::zwlr_output_head_v1::AdaptiveSyncState;
use wayland_protocols_wlr::output_management::v1::client::zwlr_output_mode_v1::ZwlrOutputModeV1;

#[allow(unused_imports)]
use log::{debug, error, info, trace, warn};

#[derive(Clone, Default, Debug, Deserialize, PartialEq, Eq)]
pub struct Position {
    pub x: i32,
    pub y: i32,
}
#[derive(Clone, Default, Debug, Deserialize, PartialEq)]
pub struct Mode {
    pub width: i32,
    pub height: i32,
    pub refresh: f32,
    #[serde(default)]
    pub custom: bool,
}
#[derive(Clone, Default, Debug, Deserialize, PartialEq)]
pub struct Output {
    pub enable: bool,
    pub r#match: String,
    pub exec: Option<Vec<String>>,
    pub mode: Option<Mode>,
    pub position: Option<Position>,
    pub scale: Option<f64>,
    #[serde(default, with = "option_transform")]
    pub transform: Option<Transform>,
    pub adaptive_sync: Option<bool>,
}
#[derive(Clone, Default, Debug, Deserialize, PartialEq)]
pub struct Profile {
    pub name: String,
    #[serde(rename = "output")]
    pub outputs: Vec<Output>,
    pub exec: Option<Vec<String>>,
}

#[derive(Clone, Debug)]
pub struct ShikaneProfilePlan {
    pub profile: Profile,
    pub config_set: Vec<OutputMatching>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct OutputMatching(pub Output, pub OutputHead, pub Option<ZwlrOutputModeV1>);

impl ShikaneProfilePlan {
    pub fn configure(
        &self,
        backend: &mut ShikaneBackend,
    ) -> Result<ZwlrOutputConfigurationV1, ShikaneError> {
        let configuration = backend.create_configuration();
        debug!("Configuring profile: {}", self.profile.name);

        for OutputMatching(output, o_head, wlr_mode) in self.config_set.iter() {
            let wlr_head = &o_head.wlr_head;
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
                if mode.custom {
                    trace!("Setting Mode: custom({})", mode);
                    configuration_head.set_custom_mode(
                        mode.width,
                        mode.height,
                        mode.refresh_m_hz(),
                    );
                } else if let Some(wlr_mode) = wlr_mode {
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

            // Transform
            if let Some(transform) = &output.transform {
                trace!("Setting Transform: {}", display_transform(transform));
                configuration_head.set_transform(*transform);
            }

            // Adaptive Sync
            if let Some(adaptive_sync) = &output.adaptive_sync {
                let as_state = match adaptive_sync {
                    true => AdaptiveSyncState::Enabled,
                    false => AdaptiveSyncState::Disabled,
                };
                if backend.wlr_output_manager_version >= 4 {
                    trace!("Setting Adaptive Sync: {as_state:?}",);
                    configuration_head.set_adaptive_sync(as_state);
                } else {
                    let msg = format!("Cannot set adaptive_sync to {as_state:?}.");
                    let msg = format!("{msg} wlr-output-management protocol version >= 4 needed.");
                    warn!("{msg} Have version {}", backend.wlr_output_manager_version);
                }
            }
        }

        Ok(configuration)
    }
}

impl PartialEq for ShikaneProfilePlan {
    fn eq(&self, other: &Self) -> bool {
        self.profile == other.profile
        // && self.config_set == other.config_set
    }
}

pub fn create_profile_plans(
    profiles: &[Profile],
    backend: &ShikaneBackend,
) -> VecDeque<ShikaneProfilePlan> {
    trace!("[Create Profile Plans]");
    let mut profile_plans = VecDeque::new();
    for profile in profiles.iter() {
        if profile.outputs.len() != backend.output_heads.len() {
            continue;
        }

        trace!("[Considering Profile] {}", profile.name);

        let o_heads = backend.heads();

        let mut edges = vec![];
        for output in profile.outputs.iter() {
            for o_head in o_heads.iter() {
                if output.matches(o_head) {
                    edges.push((output, *o_head));
                }
            }
        }

        let matchings = HKMap::new(&profile.outputs, &o_heads).create_hk_matchings(&edges);
        let config_set = create_config_set(matchings, backend);

        if config_set.len() == profile.outputs.len() {
            trace!("[Profile added to list] {}", profile.name);
            profile_plans.push_back(ShikaneProfilePlan {
                profile: profile.clone(),
                config_set,
            });
        }
    }

    profile_plans
}

fn create_config_set(
    matchings: Vec<(&Output, &OutputHead)>,
    backend: &ShikaneBackend,
) -> Vec<OutputMatching> {
    matchings
        .iter()
        .cloned()
        .filter_map(|(output, o_head)| {
            let mut mode_trace = String::new();
            let mut wlr_mode: Option<ZwlrOutputModeV1> = None;

            if let Some(mode) = &output.mode {
                // When a mode is declared custom,
                // then there is no need to look for a matching OutputMode
                if mode.custom {
                    // do nothing
                } else if let Some(o_mode) = backend.match_mode(o_head, mode) {
                    mode_trace = format!(", mode {o_mode}");
                    wlr_mode = Some(o_mode.wlr_mode.clone());
                } else {
                    // If a [`Mode`] was specified but no [`OutputMode`] matched
                    // then this profile should not be selected
                    warn!("Output {} does not support mode {mode}", o_head.name);
                    return None;
                }
            }

            trace!(
                "[Head Matched] match: {}, head.name: {}{mode_trace}",
                output.r#match,
                o_head.name,
            );

            Some(OutputMatching(output.clone(), o_head.clone(), wlr_mode))
        })
        .collect()
}

impl Mode {
    pub fn matches(&self, o_mode: &OutputMode, delta: &mut i32) -> bool {
        const MAX_DELTA: i32 = 500; // maximum difference in mHz
        let refresh: i32 = self.refresh_m_hz();
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

    /// Returns the refresh rate in mHz
    pub fn refresh_m_hz(&self) -> i32 {
        // convert Hz to mHZ and cut the decimals off
        //
        // self.refresh = 59.992_345f32  Hz
        // (_) * 1000.0 = 59_992.345f32 mHz
        // (_).trunc()  = 59_992.0f32   mHz
        // (_) as i32   = 59_992i32     mHz
        (self.refresh * 1000.0).trunc() as i32
    }
}

impl Output {
    pub fn matches(&self, o_head: &OutputHead) -> bool {
        // if a pattern is enclosed in '/' it should be interpreted as a regex
        if self.r#match.starts_with('/') && self.r#match.ends_with('/') {
            let len = self.r#match.len();
            let content = &self.r#match[1..len - 1];
            return regex::regex(content, o_head);
        }
        o_head.name == self.r#match || o_head.make == self.r#match || o_head.model == self.r#match
    }
}

mod regex {
    use super::OutputHead;

    pub fn regex(re: &str, o_head: &OutputHead) -> bool {
        let t = format!(
            "{}|{}|{}|{}|{}",
            o_head.name, o_head.make, o_head.model, o_head.serial_number, o_head.description
        );
        match regex::Regex::new(re) {
            Ok(regex) if regex.is_match(&t) => {
                log::debug!("[Matched] \"{:?}\" {t:?}", regex);
                return true;
            }
            Ok(regex) => {
                log::debug!("[Does not match] \"{:?}\" {t:?}", regex);
            }
            Err(err) => log::warn!("[Error] {}", err),
        }
        false
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

// `Transform` helpers
#[derive(Deserialize)]
#[serde(remote = "Transform")]
#[repr(u32)]
#[non_exhaustive]
enum TransformDef {
    #[serde(rename = "normal")]
    Normal,
    #[serde(rename = "90")]
    _90,
    #[serde(rename = "180")]
    _180,
    #[serde(rename = "270")]
    _270,
    #[serde(rename = "flipped")]
    Flipped,
    #[serde(rename = "flipped-90")]
    Flipped90,
    #[serde(rename = "flipped-180")]
    Flipped180,
    #[serde(rename = "flipped-270")]
    Flipped270,
}

mod option_transform {
    use super::{Transform, TransformDef};
    use serde::{Deserialize, Deserializer};

    // see https://github.com/serde-rs/serde/issues/1301#issuecomment-394108486
    pub fn deserialize<'de, D>(deserializer: D) -> Result<Option<Transform>, D::Error>
    where
        D: Deserializer<'de>,
    {
        #[derive(Deserialize)]
        struct Helper(#[serde(with = "TransformDef")] Transform);

        let helper = Option::deserialize(deserializer)?;
        Ok(helper.map(|Helper(external)| external))
    }
}

fn display_transform(t: &Transform) -> String {
    match t {
        Transform::Normal => "normal",
        Transform::_90 => "90",
        Transform::_180 => "180",
        Transform::_270 => "270",
        Transform::Flipped => "flipped",
        Transform::Flipped90 => "flipped-90",
        Transform::Flipped180 => "flipped-180",
        Transform::Flipped270 => "flipped-270",
        _ => "",
    }
    .to_string()
}
