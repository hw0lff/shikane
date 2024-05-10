pub mod ipc;
pub mod profile_manager;
pub mod state_machine;

use std::os::unix::net::UnixListener;
use std::path::PathBuf;

use calloop::EventLoop;
use clap::Parser;
#[allow(unused_imports)]
use log::{debug, error, info, trace, warn};
use snafu::{prelude::*, Location};

use crate::daemon::state_machine::DaemonStateMachine;
use crate::error;
use crate::ipc::SocketBindCtx;
use crate::settings::Settings;
use crate::wl_backend::WlBackend;
use crate::wlroots::WlrootsBackend;

type DSMWlroots = DaemonStateMachine<WlrootsBackend>;

#[derive(Debug, Parser)]
#[command(version)]
pub struct Shikane {
    /// Path to config file
    #[arg(short, long, value_name = "PATH")]
    pub config: Option<PathBuf>,

    /// Enable oneshot mode
    ///
    /// Exit after a profile has been applied or
    /// if no profile was matched
    #[arg(short, long)]
    pub oneshot: bool,

    /// IPC socket path
    #[arg(short, long, value_name = "PATH")]
    pub socket: Option<PathBuf>,

    /// Apply profiles untested
    #[arg(short = 't', long, value_parser = clap::builder::BoolishValueParser::new(), action = clap::ArgAction::Set, default_value = "false", hide = true)]
    pub skip_tests: bool,
}

pub fn daemon(args: Option<Shikane>) {
    let args = match args {
        Some(args) => args,
        None => Shikane::parse(),
    };
    if let Err(err) = run(args) {
        error!("{}", error::report(err.as_ref()))
    }
}

fn run(args: Shikane) -> Result<(), Box<dyn snafu::Error>> {
    let arg_socket_path = args.socket.clone();
    let settings = Settings::from_args(args);

    let (wlroots_backend, wl_source) = WlrootsBackend::connect()?;
    let mut dsm = DSMWlroots::new(wlroots_backend, settings);
    let mut event_loop: EventLoop<DSMWlroots> = EventLoop::try_new().context(ELCreateCtx)?;
    let el_handle = event_loop.handle();
    let loop_signal = event_loop.get_signal();

    let socket_path = match arg_socket_path {
        Some(path) => path,
        None => crate::util::get_socket_path()?,
    };
    clean_up_socket(&socket_path);
    trace!("Binding socket to {:?}", socket_path.to_string_lossy());
    let listener = UnixListener::bind(&socket_path).context(SocketBindCtx { path: socket_path })?;

    let socket_event_source =
        calloop::generic::Generic::new(listener, calloop::Interest::READ, calloop::Mode::Level);

    el_handle
        .insert_source(wl_source, move |_, event_queue, state| {
            let dispatch_result = event_queue.dispatch_pending(&mut state.backend);
            let n = match dispatch_result {
                Ok(n) => n,
                Err(ref err) => {
                    error!("{}", error::report(&err));
                    return dispatch_result;
                }
            };
            trace!("dispatched {n} wayland events");
            trace!("processing event queue");
            let sm_shutdown = state.process_event_queue();
            if sm_shutdown {
                trace!("stopping event loop");
                loop_signal.stop();
            }
            dispatch_result
        })
        .context(InsertCtx)?;

    let el_handle2 = el_handle.clone();
    el_handle
        .insert_source(socket_event_source, move |_, listener, _| {
            if let Err(err) = ipc::handle_listener(listener, &el_handle2) {
                let err = error::report(&*err);
                warn!("{err}");
            }
            Ok(calloop::PostAction::Continue)
        })
        .context(InsertCtx)?;

    let loop_signal = event_loop.get_signal();
    event_loop
        .run(
            std::time::Duration::from_millis(500),
            &mut dsm,
            |state| match state.backend.flush() {
                Ok(_) => {}
                Err(err) => {
                    error!("backend error on flush: {}", error::report(&err));
                    loop_signal.stop();
                }
            },
        )
        .context(RunCtx)?;
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

#[derive(Debug, Snafu)]
#[snafu(context(suffix(Ctx)))]
#[snafu(visibility(pub(crate)))]
pub(crate) enum EventLoopSetupError {
    #[snafu(display("[{location}] An error occurred while running the event loop"))]
    Run {
        source: calloop::Error,
        location: Location,
    },
    #[snafu(display("[{location}] Cannot create new event loop"))]
    ELCreate {
        source: calloop::Error,
        location: Location,
    },
}

#[derive(Debug, Snafu)]
#[snafu(context(suffix(Ctx)))]
#[snafu(visibility(pub(crate)))]
pub(crate) enum EventLoopInsertError<T>
where
    T: 'static,
{
    #[snafu(display("[{location}] Cannot insert event source into event loop"))]
    Insert {
        source: calloop::InsertError<T>,
        location: Location,
    },
}
