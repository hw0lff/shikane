use std::{fs, path::PathBuf};

use log::{debug, trace};
use serde::Deserialize;
use xdg::BaseDirectories;

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
}
#[derive(Default, Debug, Deserialize)]
pub(crate) struct ShikaneConfig {
    #[serde(rename = "profile")]
    pub(crate) profiles: Vec<Profile>,
}

impl ShikaneConfig {
    pub(crate) fn parse(config_path: Option<PathBuf>) -> ShikaneConfig {
        let config_path = match config_path {
            None => {
                let xdg_dirs =
                    BaseDirectories::with_prefix("shikane").expect("failed to get xdg directories");
                xdg_dirs
                    .place_config_file("config.toml")
                    .expect("cannot create configuration directory")
            }
            Some(path) => path,
        };
        let s = fs::read_to_string(config_path).expect("cannot read config file");
        let config = toml::from_str(&s).expect("cannot parse config file");
        debug!("Config file parsed");
        trace!("Config: {:#?}", config);
        config
    }
}
