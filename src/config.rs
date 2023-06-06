use std::{fs, path::PathBuf};

use log::trace;
use serde::Deserialize;
use xdg::BaseDirectories;

use crate::error::ShikaneError;
use crate::profile::Profile;

#[derive(Default, Debug, Deserialize)]
pub struct ShikaneConfig {
    #[serde(default, rename = "profile")]
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
