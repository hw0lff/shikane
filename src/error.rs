use thiserror::Error;

#[derive(Error, Debug)]
pub enum ShikaneError {
    #[error("Cannot apply configuration")]
    ConfigurationError,
    #[error(transparent)]
    EventLoop(#[from] ::calloop::error::Error),
    #[error(transparent)]
    Io(#[from] std::io::Error),
    #[error(transparent)]
    TomlSerde(#[from] toml::de::Error),
    #[error("Unable to release resources associated with destroyed mode")]
    ReleaseOutputMode,
    #[error(transparent)]
    WaylandBackend(#[from] ::wayland_client::backend::WaylandError),
    #[error(transparent)]
    WaylandConnection(#[from] ::wayland_client::ConnectError),
    #[error("Cannot get wayland object from specified ID")]
    WaylandInvalidId(#[from] ::wayland_client::backend::InvalidId),
    #[error(transparent)]
    Xdg(#[from] ::xdg::BaseDirectoriesError),
}

impl<T> From<calloop::InsertError<T>> for ShikaneError {
    fn from(err: calloop::InsertError<T>) -> Self {
        ShikaneError::EventLoop(err.into())
    }
}
