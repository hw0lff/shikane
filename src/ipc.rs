use std::io::{Read, Write};
use std::os::unix::net::UnixStream;

use clap::Subcommand;
#[allow(unused_imports)]
use log::{debug, error, info, trace, warn};
use serde::{Deserialize, Serialize};
use snafu::ResultExt;

use crate::error::{self, ShikaneSocketError};

#[derive(Clone, Debug, Deserialize, Serialize, Subcommand)]
pub(crate) enum ShikaneCommand {
    Debug,
}

pub(crate) fn recv_data(stream: &mut UnixStream) -> Result<String, ShikaneSocketError> {
    trace!("[Reading data from socket]");
    let mut data = String::new();
    stream
        .read_to_string(&mut data)
        .context(error::SocketReadCtx)?;

    let direction = std::net::Shutdown::Read;
    shutdown_socket(stream, direction)?;
    Ok(data)
}

pub(crate) fn send_data(stream: &mut UnixStream, data: String) -> Result<(), ShikaneSocketError> {
    trace!("[Writing data to socket]");
    stream
        .write_all(data.as_bytes())
        .context(error::SocketWriteCtx)?;

    let direction = std::net::Shutdown::Write;
    shutdown_socket(stream, direction)?;
    Ok(())
}

pub(crate) fn shutdown_socket(
    stream: &mut UnixStream,
    direction: std::net::Shutdown,
) -> Result<(), ShikaneSocketError> {
    trace!("[Shutdown socket] direction: {direction:?}");
    stream
        .shutdown(direction)
        .context(error::ShutdownCtx { direction })
}
