use std::{fs, path::PathBuf};

use log::trace;
use serde::Deserialize;
use xdg::BaseDirectories;

use crate::error::ShikaneError;

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
#[derive(Default, Debug, Deserialize)]
pub struct ShikaneConfig {
    #[serde(rename = "profile")]
    pub profiles: Vec<Profile>,
}

impl ShikaneConfig {
    pub fn parse(config_path: Option<PathBuf>) -> Result<ShikaneConfig, ShikaneError> {
        let config_path = match config_path {
            None => {
                let xdg_dirs = BaseDirectories::with_prefix("shikane")?;
                xdg_dirs.place_config_file("config.toml")?
            }
            Some(path) => path,
        };
        let s = fs::read_to_string(config_path)?;
        let config = toml::from_str(&s)?;
        trace!("Config file parsed");
        Ok(config)
    }
}
