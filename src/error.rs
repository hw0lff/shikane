use thiserror::Error;

#[derive(Error, Debug)]
pub enum ShikaneError {
    #[error("Configuration: Cannot configure profile {0:?}")]
    ConfigurationError(String),
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
