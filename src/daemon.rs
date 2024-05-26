pub mod ipc;
pub mod profile_manager;
pub mod state_machine;

use std::collections::VecDeque;
use std::os::unix::net::UnixListener;
use std::path::PathBuf;
use std::time::Duration;

use calloop::{timer::Timer, EventLoop, LoopHandle, LoopSignal, RegistrationToken};
use clap::Parser;
#[allow(unused_imports)]
use log::{debug, error, info, trace, warn};
use snafu::{prelude::*, Location};

use crate::daemon::state_machine::DaemonStateMachine;
use crate::error;
use crate::ipc::SocketBindCtx;
use crate::settings::Settings;
use crate::wl_backend::{WlBackend, WlBackendEvent};
use crate::wlroots::WlrootsBackend;

type DSMWlroots = DaemonStateMachine<WlrootsBackend>;

#[derive(Debug, Parser)]
#[command(version)]
pub struct ShikaneArgs {
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

    /// Wait for TIMEOUT milliseconds before processing changes
    ///
    /// Usually you should not set this as it slows down shikane.
    #[arg(short = 'T', long, default_value_t = 0)]
    pub timeout: u64,
}

pub struct Shikane<'a, B: WlBackend> {
    dsm: DaemonStateMachine<B>,
    event_queue: VecDeque<WlBackendEvent>,
    el_handle: LoopHandle<'a, Shikane<'a, B>>,
    loop_signal: LoopSignal,
    timeout_token: RegistrationToken,
}

pub fn daemon(args: Option<ShikaneArgs>) {
    let args = match args {
        Some(args) => args,
        None => ShikaneArgs::parse(),
    };
    if let Err(err) = run(args) {
        error!("{}", error::report(err.as_ref()))
    }
}

fn run(args: ShikaneArgs) -> Result<(), Box<dyn snafu::Error>> {
    let arg_socket_path = args.socket.clone();
    let settings = Settings::from_args(args);

    let (wlroots_backend, wl_source) = WlrootsBackend::connect()?;
    let mut event_loop: EventLoop<Shikane<WlrootsBackend>> =
        EventLoop::try_new().context(ELCreateCtx)?;
    let el_handle = event_loop.handle();

    // wayland backend
    el_handle
        .insert_source(wl_source, move |_, event_queue, shikane| {
            let dispatch_result = event_queue.dispatch_pending(&mut shikane.dsm.backend);
            let n = match dispatch_result {
                Ok(n) => n,
                Err(ref err) => {
                    error!("{}", error::report(&err));
                    return dispatch_result;
                }
            };
            trace!("dispatched {n} wayland events");

            let mut eq = shikane.dsm.backend.drain_event_queue();
            shikane.event_queue.append(&mut eq);
            let mut timeout = Duration::from_millis(0);
            // delay processing only on change
            if shikane
                .event_queue
                .contains(&WlBackendEvent::AtomicChangeDone)
            {
                timeout = shikane.dsm.settings.timeout;
                trace!("delay processing by {:?}", timeout);
            }
            trace!(
                "inserting new {:?} timer, replacing old {:?}",
                timeout,
                shikane.timeout_token
            );
            shikane.el_handle.remove(shikane.timeout_token);
            match insert_timer(&shikane.el_handle, timeout) {
                Ok(token) => shikane.timeout_token = token,
                Err(err) => error!("{}", error::report(&err)),
            }
            dispatch_result
        })
        .context(InsertCtx)?;

    // IPC socket
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
        .insert_source(socket_event_source, move |_, listener, shikane| {
            if let Err(err) = ipc::handle_listener(listener, &shikane.el_handle) {
                let err = error::report(&*err);
                warn!("{err}");
            }
            Ok(calloop::PostAction::Continue)
        })
        .context(InsertCtx)?;

    // initial timeout
    let timeout_token = insert_timer(&el_handle, settings.timeout)?;

    let dsm = DSMWlroots::new(wlroots_backend, settings);
    let loop_signal = event_loop.get_signal();
    let mut shikane = Shikane {
        dsm,
        event_queue: Default::default(),
        el_handle,
        loop_signal,
        timeout_token,
    };
    event_loop
        .run(
            std::time::Duration::from_millis(500),
            &mut shikane,
            |shikane| match shikane.dsm.backend.flush() {
                Ok(_) => {}
                Err(err) => {
                    error!("backend error on flush: {}", error::report(&err));
                    shikane.loop_signal.stop()
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

fn insert_timer<B: WlBackend>(
    el_handle: &LoopHandle<Shikane<B>>,
    duration: Duration,
) -> Result<RegistrationToken, EventLoopInsertError<Timer>> {
    let timer = Timer::from_duration(duration);
    let timeout_token = el_handle
        .insert_source(timer, move |_instant, _, shikane| {
            trace!("processing event queue");
            let eq = std::mem::take(&mut shikane.event_queue);
            let sm_shutdown = shikane.dsm.process_event_queue(eq);
            if sm_shutdown {
                trace!("stopping event loop");
                shikane.loop_signal.stop();
            }
            calloop::timer::TimeoutAction::Drop
        })
        .context(InsertCtx)?;
    Ok(timeout_token)
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
