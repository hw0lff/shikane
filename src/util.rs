#[allow(unused_imports)]
use log::{debug, error, info, trace, warn};
use snafu::{prelude::*, Location};

const SHIKANE_LOG_DEFAULT: &str = "warn,shikane::variant=info,shikane::daemon::state_machine=info";

pub fn setup_logging() {
    let log_time: Option<env_logger::TimestampPrecision> =
        match std::env::var("SHIKANE_LOG_TIME").is_ok_and(|value| value.trim() == "1") {
            true => Some(env_logger::TimestampPrecision::Millis),
            false => None,
        };

    env_logger::Builder::from_env(
        env_logger::Env::new()
            .filter_or("SHIKANE_LOG", SHIKANE_LOG_DEFAULT)
            .write_style_or("SHIKANE_LOG_STYLE", "auto"),
    )
    .format_timestamp(log_time)
    .init();
}

pub(crate) fn get_socket_path() -> Result<std::path::PathBuf, UtilError> {
    let wayland_display = "WAYLAND_DISPLAY";
    let wayland_display = std::env::var(wayland_display).context(EnvVarCtx {
        var: wayland_display,
    })?;

    let xdg_dirs = xdg::BaseDirectories::new().context(BaseDirectoriesCtx)?;

    let path = format!("shikane-{wayland_display}.socket");
    let path = xdg_dirs.place_runtime_file(path).context(SocketPathCtx)?;
    Ok(path)
}

#[derive(Debug, Snafu)]
#[snafu(context(suffix(Ctx)))]
#[snafu(visibility(pub(crate)))]
pub enum UtilError {
    #[snafu(display("[{location}] Problem with XDG directories"))]
    BaseDirectories {
        source: xdg::BaseDirectoriesError,
        location: Location,
    },
    #[snafu(display("[{location}] Cannot find environment variable {var}"))]
    EnvVar {
        source: std::env::VarError,
        location: Location,
        var: String,
    },
    #[snafu(display("[{location}] Cannot place socket in XDG runtime directory"))]
    SocketPath {
        source: std::io::Error,
        location: Location,
    },
}
