use crate::backend::ShikaneBackend;
use crate::config::ShikaneConfig;
use crate::error::{self, ShikaneError, ShikaneRonError};
use crate::ipc::{self, ShikaneCommand};
use crate::state::{self, ShikaneState};

use calloop::{channel, EventLoop, LoopHandle};
use clap::Parser;
use snafu::prelude::*;

#[allow(unused_imports)]
use log::{debug, error, info, trace, warn};

use std::os::unix::net::{UnixListener, UnixStream};
use std::path::PathBuf;

#[derive(Debug, Parser)]
#[command(version)]
pub struct ShikaneDaemonArgs {
    /// Path to config file
    #[arg(short, long, value_name = "PATH")]
    pub config: Option<PathBuf>,

    /// Enable oneshot mode
    ///
    /// Exit after a profile has been applied or
    /// if no profile was matched
    #[arg(short, long)]
    pub oneshot: bool,

    /// Apply profiles untested
    #[arg(short, long, value_parser = clap::builder::BoolishValueParser::new(), action = clap::ArgAction::Set, default_value = "true", hide = true)]
    pub skip_tests: bool,
}

pub fn daemon(args: Option<ShikaneDaemonArgs>) {
    if let Err(err) = run(args) {
        error!("{}", err)
    }
}

fn run(args: Option<ShikaneDaemonArgs>) -> Result<(), ShikaneError> {
    let args = match args {
        Some(args) => args,
        None => ShikaneDaemonArgs::parse(),
    };
    let config = ShikaneConfig::parse(args.config.clone())?;

    let mut event_loop: EventLoop<ShikaneState> = EventLoop::try_new()?;
    let (sender, channel) = channel::channel();
    let (backend, wl_source) = ShikaneBackend::connect(sender)?;
    let mut state = ShikaneState::new(args, backend, config, event_loop.get_signal());
    let el_handle = event_loop.handle();

    let socket_path = crate::util::get_socket_path()?;
    clean_up_socket(&socket_path);
    trace!("Binding socket to {:?}", socket_path.to_string_lossy());
    let listener = UnixListener::bind(socket_path)?;

    let socket_event_source =
        calloop::generic::Generic::new(listener, calloop::Interest::READ, calloop::Mode::Level);

    el_handle.insert_source(channel, |event, _, state| match event {
        channel::Event::Msg(m) => state.advance(m),
        channel::Event::Closed => todo!(),
    })?;

    el_handle.insert_source(wl_source, |_, event_queue, state| {
        state.backend.callback(event_queue)
    })?;

    let el_handle2 = el_handle.clone();
    el_handle.insert_source(socket_event_source, move |_, listener, _| {
        if let Err(err) = handle_listener(listener, &el_handle2) {
            let err = error::report(&err);
            warn!("{err}");
        }
        Ok(calloop::PostAction::Continue)
    })?;

    let el_signal = event_loop.get_signal();
    // Idle timeout callback
    event_loop.run(
        std::time::Duration::from_millis(500),
        &mut state,
        |state| match state.idle() {
            Ok(_) => {}
            Err(err) => {
                error!("{}", err);
                el_signal.stop();
            }
        },
    )?;
    Ok(())
}

fn clean_up_socket(socket_path: &PathBuf) {
    match std::fs::remove_file(socket_path) {
        Ok(_) => trace!("Deleted stale socket from previous run"),
        Err(err) if err.kind() == std::io::ErrorKind::NotFound => {}
        Err(err) => {
            warn!("Cannot delete socket: {}", err)
        }
    }
}

fn handle_listener(
    listener: &mut UnixListener,
    el_handle: &LoopHandle<ShikaneState>,
) -> Result<(), ShikaneError> {
    let (stream, _) = listener.accept()?;
    let stream_event_source =
        calloop::generic::Generic::new(stream, calloop::Interest::BOTH, calloop::Mode::Edge);

    trace!("[EventLoop] Inserting IPC client");
    el_handle.insert_source(stream_event_source, |_, stream, state| {
        if let Err(err) = handle_client(stream, state) {
            let err = error::report(err.as_ref());
            warn!("IPC error({})", err);
        }
        Ok(calloop::PostAction::Remove)
    })?;
    Ok(())
}

fn handle_client(
    stream: &mut UnixStream,
    state: &mut ShikaneState,
) -> Result<(), Box<dyn snafu::Error>> {
    let command = ipc::recv_data(stream)?;
    let command = parse_command(command)?;
    let answer = process_command(command, state);
    ipc::send_data(stream, answer)?;
    Ok(())
}

fn parse_command(command: String) -> Result<ShikaneCommand, ShikaneRonError> {
    let command: ShikaneCommand = ron::from_str(&command).context(error::RonDeserializeCtx)?;
    debug!("[Parsed command] {command:?}");
    Ok(command)
}

fn process_command(command: ShikaneCommand, state: &mut ShikaneState) -> String {
    match command {
        ShikaneCommand::Debug => cmd_debug(state),
    }
}

fn cmd_debug(state: &ShikaneState) -> String {
    match state.state {
        state::State::ProfileApplied(ref pp) => {
            format!("current profile: {}", pp.profile.name)
        }
        _ => String::new(),
    }
}
