use std::{fs, path::PathBuf};

use log::{debug, trace};
use serde::Deserialize;
use xdg::BaseDirectories;

use crate::error::ShikaneError;

#[derive(Clone, Default, Debug, Deserialize)]
pub(crate) struct Position {
    pub(crate) x: i32,
    pub(crate) y: i32,
}
#[derive(Clone, Default, Debug, Deserialize)]
pub(crate) struct Mode {
    pub(crate) width: i32,
    pub(crate) height: i32,
    pub(crate) refresh: i32,
}
#[derive(Clone, Default, Debug, Deserialize)]
pub(crate) struct Output {
    pub(crate) enable: bool,
    pub(crate) r#match: String,
    pub(crate) mode: Mode,
    pub(crate) position: Position,
}
#[derive(Clone, Default, Debug, Deserialize)]
pub(crate) struct Profile {
    pub(crate) name: String,
    #[serde(rename = "output")]
    pub(crate) outputs: Vec<Output>,
    pub(crate) exec: Option<Vec<String>>,
}
#[derive(Default, Debug, Deserialize)]
pub(crate) struct ShikaneConfig {
    #[serde(rename = "profile")]
    pub(crate) profiles: Vec<Profile>,
}

impl ShikaneConfig {
    pub(crate) fn parse(config_path: Option<PathBuf>) -> Result<ShikaneConfig, ShikaneError> {
        let config_path = match config_path {
            None => {
                let xdg_dirs = BaseDirectories::with_prefix("shikane")?;
                xdg_dirs.place_config_file("config.toml")?
            }
            Some(path) => path,
        };
        let s = fs::read_to_string(config_path)?;
        let config = toml::from_str(&s)?;
        debug!("Config file parsed");
        trace!("Config: {:#?}", config);
        Ok(config)
    }
}
