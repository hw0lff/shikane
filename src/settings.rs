use std::collections::VecDeque;
use std::path::PathBuf;
use std::time::Duration;

#[allow(unused_imports)]
use log::{debug, error, info, trace, warn};
use serde::{Deserialize, Serialize};
use snafu::{prelude::*, Location};
use xdg::BaseDirectories;

use crate::daemon::ShikaneArgs;
use crate::error;
use crate::profile::Profile;

#[derive(Clone, Debug)]
pub struct Settings {
    pub profiles: VecDeque<Profile>,
    pub skip_tests: bool,
    pub oneshot: bool,
    pub timeout: Duration,
    pub config_path: PathBuf,
}

#[derive(Default, Debug, Serialize, Deserialize)]
pub struct SettingsToml {
    pub timeout: Option<u64>,
    #[serde(default, rename = "profile")]
    pub profiles: VecDeque<Profile>,
}

impl Settings {
    pub fn from_args(args: ShikaneArgs) -> Self {
        let (config, path) = match parse_settings_toml(args.config) {
            Ok(config) => config,
            Err(err) => {
                error!("{}", error::report(err.as_ref()));
                std::process::exit(1);
            }
        };

        let timeout = config.timeout.unwrap_or(args.timeout);

        Self {
            profiles: config.profiles,
            skip_tests: args.skip_tests,
            oneshot: args.oneshot,
            timeout: Duration::from_millis(timeout),
            config_path: path,
        }
    }

    pub fn reload_config(&mut self, config: Option<PathBuf>) -> Result<(), Box<dyn snafu::Error>> {
        let config = config.unwrap_or(self.config_path.clone());
        debug!("reloading config from {:?}", std::fs::canonicalize(&config));
        let (config, path) = parse_settings_toml(Some(config))?;
        self.profiles = config.profiles;
        self.config_path = path;
        Ok(())
    }
}

fn parse_settings_toml(
    config_path: Option<PathBuf>,
) -> Result<(SettingsToml, PathBuf), Box<dyn snafu::Error>> {
    let config_path = match config_path {
        None => {
            let xdg_dirs = BaseDirectories::with_prefix("shikane").context(BaseDirectoriesCtx)?;
            xdg_dirs
                .place_config_file("config.toml")
                .context(ConfigPathCtx)?
        }
        Some(path) => path,
    };
    let s = std::fs::read_to_string(config_path.clone()).context(ReadConfigFileCtx)?;
    let mut config: SettingsToml = toml::from_str(&s).context(TomlDeserializeCtx)?;
    config
        .profiles
        .iter_mut()
        .enumerate()
        .for_each(|(idx, p)| p.index = idx);
    Ok((config, config_path))
}

#[derive(Debug, Snafu)]
#[snafu(context(suffix(Ctx)))]
pub enum SettingsError {
    #[snafu(display("[{location}] Problem with XDG directories"))]
    BaseDirectories {
        source: xdg::BaseDirectoriesError,
        location: Location,
    },
    #[snafu(display("[{location}] Cannot read config file"))]
    ReadConfigFile {
        source: std::io::Error,
        location: Location,
    },
    #[snafu(display("[{location}] Cannot place config file in XDG config directory"))]
    ConfigPath {
        source: std::io::Error,
        location: Location,
    },
    #[snafu(display("[{location}] Cannot deserialize settings from TOML"))]
    TomlDeserialize {
        source: toml::de::Error,
        location: Location,
    },
}
