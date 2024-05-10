use std::collections::VecDeque;
use std::io::{Read, Write};
use std::net::Shutdown;
use std::os::fd::AsRawFd;
use std::os::unix::net::UnixStream;
use std::path::PathBuf;

#[allow(unused_imports)]
use log::{debug, error, info, trace, warn};
use serde::{de::DeserializeOwned, Deserialize, Serialize};
use snafu::{prelude::*, Location};

use crate::matching::MatchReport;
use crate::wl_backend::WlHead;

#[derive(Clone, Debug, Deserialize, Serialize)]
pub(crate) enum IpcRequest {
    CurrentHeads,
    CurrentState,
    MatchReports,
    ReloadConfig(Option<PathBuf>),
    SwitchProfile(String),
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub(crate) enum IpcResponse {
    CurrentHeads(VecDeque<WlHead>),
    Error(String),
    Generic(String),
    MatchReports(VecDeque<MatchReport>),
    Success,
}

#[derive(Debug)]
pub(crate) struct IpcStream {
    stream: UnixStream,
}

impl IpcStream {
    pub(crate) fn connect() -> Result<Self, IpcSetupError> {
        let socket = get_socket_path()?;
        Self::connect_to(&socket)
    }

    pub(crate) fn connect_to(socket: &PathBuf) -> Result<Self, IpcSetupError> {
        let stream = UnixStream::connect(socket).context(SocketConnectCtx)?;
        Ok(Self::from(stream))
    }

    pub(crate) fn recv<D: DeserializeOwned>(&mut self) -> Result<D, IpcError> {
        trace!("[reading data from socket]");
        let mut buf = String::new();
        self.stream
            .read_to_string(&mut buf)
            .context(SocketReadCtx)?;
        let data = ron::de::from_str(&buf).context(RonDeserializeCtx)?;
        self.shutdown_ipc_socket(Shutdown::Read)?;
        Ok(data)
    }

    pub(crate) fn send<D: Serialize>(&mut self, data: &D) -> Result<(), IpcError> {
        let buf = ron::ser::to_string(&data).context(RonSerializeCtx)?;
        trace!("[writing data to socket] {:?}", buf);
        self.stream
            .write_all(buf.as_bytes())
            .context(SocketWriteCtx)?;
        self.shutdown_ipc_socket(Shutdown::Write)?;
        Ok(())
    }

    pub(crate) fn shutdown_ipc_socket(&mut self, direction: Shutdown) -> Result<(), IpcError> {
        trace!("[shutdown socket] direction: {direction:?}");
        self.stream
            .shutdown(direction)
            .context(ShutdownCtx { direction })
    }

    pub(crate) fn into_event_source(self) -> calloop::generic::Generic<Self> {
        self.into()
    }
}

pub(crate) fn get_socket_path() -> Result<PathBuf, IpcSetupError> {
    let wayland_display = "WAYLAND_DISPLAY";
    let wayland_display = std::env::var("WAYLAND_DISPLAY").context(EnvVarCtx {
        var: wayland_display,
    })?;

    let xdg_dirs = xdg::BaseDirectories::new().context(BaseDirectoriesCtx)?;

    let path = format!("shikane-{wayland_display}.socket");
    let path = xdg_dirs.place_runtime_file(path).context(SocketPathCtx)?;
    Ok(path)
}

impl From<IpcStream> for calloop::generic::Generic<IpcStream> {
    fn from(value: IpcStream) -> Self {
        calloop::generic::Generic::new(value, calloop::Interest::BOTH, calloop::Mode::Edge)
    }
}

impl AsRawFd for IpcStream {
    fn as_raw_fd(&self) -> std::os::unix::prelude::RawFd {
        self.stream.as_raw_fd()
    }
}

impl From<UnixStream> for IpcStream {
    fn from(value: UnixStream) -> Self {
        Self { stream: value }
    }
}

#[derive(Debug, Snafu)]
#[snafu(context(suffix(Ctx)))]
#[snafu(visibility(pub(crate)))]
pub(crate) enum IpcError {
    // Socket
    #[snafu(display("[{location}] Failed to accept incoming socket connection"))]
    SocketAccept {
        source: std::io::Error,
        location: Location,
    },
    #[snafu(display("[{location}] Cannot bind to socket {path:?}"))]
    SocketBind {
        source: std::io::Error,
        location: Location,
        path: PathBuf,
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
        direction: Shutdown,
    },
    // SerDe
    #[snafu(display("[{location}] Cannot serialize data to RON"))]
    RonSerialize {
        source: ron::error::Error,
        location: Location,
    },
    #[snafu(display("[{location}] Cannot deserialize data from RON"))]
    RonDeserialize {
        source: ron::error::SpannedError,
        location: Location,
    },
}

#[derive(Debug, Snafu)]
#[snafu(context(suffix(Ctx)))]
pub(crate) enum IpcSetupError {
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
    #[snafu(display("[{location}] Cannot connect to socket"))]
    SocketConnect {
        source: std::io::Error,
        location: Location,
    },
}
