use snafu::{prelude::*, Location};
use thiserror::Error;

#[derive(Error, Debug)]
pub enum ShikaneError {
    #[error("Configuration: Cannot configure profile {0:?}")]
    Configuration(String),
    #[error("EventLoop: {0}")]
    EventLoop(#[from] ::calloop::error::Error),
    #[error("Io: {0}")]
    Io(#[from] std::io::Error),
    #[error("TomlSerde: {0}")]
    TomlSerde(#[from] toml::de::Error),
    #[error("Unable to release resources associated with destroyed mode")]
    ReleaseOutputMode,
    #[error("WaylandBackend: {0}")]
    WaylandBackend(#[from] ::wayland_client::backend::WaylandError),
    #[error("WaylandConnection: {0}")]
    WaylandConnection(#[from] ::wayland_client::ConnectError),
    #[error("Cannot get wayland object from specified ID: {0}")]
    WaylandInvalidId(#[from] ::wayland_client::backend::InvalidId),
    #[error("Xdg: {0}")]
    Xdg(#[from] ::xdg::BaseDirectoriesError),
}

impl<T> From<calloop::InsertError<T>> for ShikaneError {
    fn from(err: calloop::InsertError<T>) -> Self {
        ShikaneError::EventLoop(err.into())
    }
}

#[derive(Debug, Snafu)]
#[snafu(context(suffix(Ctx)))]
#[snafu(visibility(pub(crate)))]
pub(crate) enum ShikaneSocketError {
    #[snafu(display("[{location}] Cannot connect to socket"))]
    SocketConnect {
        source: std::io::Error,
        location: Location,
    },
    #[snafu(display("[{location}] Cannot read from socket"))]
    SocketRead {
        source: std::io::Error,
        location: Location,
    },
    #[snafu(display("[{location}] Cannot write to socket"))]
    SocketWrite {
        source: std::io::Error,
        location: Location,
    },
    #[snafu(display("[{location}] Cannot shutdown stream for {direction:?} directon(s)"))]
    Shutdown {
        source: std::io::Error,
        location: Location,
        direction: std::net::Shutdown,
    },
}

pub(crate) fn report(error: &dyn snafu::Error) -> String {
    let sources = snafu::ChainCompat::new(error);
    let sources: Vec<&dyn snafu::Error> = sources.collect();
    let sources = sources.iter().rev();
    let mut s = String::new();
    for (i, source) in sources.enumerate() {
        s = match i {
            0 => format!("{source}"),
            _ => format!("{source} ({s})"),
        }
    }
    s
}
