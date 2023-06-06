use crate::error::ShikaneSocketError;
use crate::ipc::{self, ShikaneCommand};
use crate::{error, util};

use clap::Parser;
use snafu::prelude::*;

#[allow(unused_imports)]
use log::{debug, error, info, trace, warn};

use std::os::unix::net::UnixStream;
use std::path::PathBuf;

#[derive(Debug, Parser)]
#[command(version)]
struct ShikaneClientArgs {
    #[command(subcommand)]
    cmd: ShikaneCommand,
}

pub fn client() {
    if let Err(err) = run() {
        error!("{}", error::report(err.as_ref()))
    }
}

fn run() -> Result<(), Box<dyn snafu::Error>> {
    let args = ShikaneClientArgs::parse();
    let command = ron::to_string(&args.cmd).context(error::RonSerializeCtx)?;

    let socket_path = util::get_socket_path()?;
    let mut stream = connect_to_socket(socket_path)?;
    send_command(&mut stream, command)?;
    let answer = recv_answer_from_daemon(&mut stream)?;
    println!("{answer}");

    Ok(())
}

fn connect_to_socket(socket_path: PathBuf) -> Result<UnixStream, ShikaneSocketError> {
    trace!("Connecting to daemon at {socket_path:?}");
    UnixStream::connect(socket_path).context(error::SocketConnectCtx)
}

fn send_command(stream: &mut UnixStream, command: String) -> Result<(), ShikaneSocketError> {
    debug!("[Sending command] {command}");
    ipc::send_data(stream, command)
}

fn recv_answer_from_daemon(stream: &mut UnixStream) -> Result<String, ShikaneSocketError> {
    trace!("[IPC] Receiving answer");
    ipc::recv_data(stream)
}
