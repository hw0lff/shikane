use crate::backend::ShikaneBackend;
use crate::error::ShikaneError;

use serde::Deserialize;
use wayland_protocols_wlr::output_management::v1::client::zwlr_output_configuration_v1::ZwlrOutputConfigurationV1;

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
            .match_head(&output.r#match)
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
            .match_mode(head_id, &output.mode)
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
