use std::os::unix::net::UnixListener;
use std::path::PathBuf;

use calloop::LoopHandle;
#[allow(unused_imports)]
use log::{debug, error, info, trace, warn};
use snafu::prelude::*;

use crate::daemon::InsertCtx;
use crate::error;
use crate::ipc::{IpcRequest, IpcResponse, IpcStream, SocketAcceptCtx};
use crate::search::SearchPattern;
use crate::wl_backend::WlBackend;

use super::profile_manager::{ProfileManager, Restriction};
use super::state_machine::{DSMState, DaemonStateMachine};
use super::Shikane;

type Dsm<T> = DaemonStateMachine<T>;

pub fn handle_listener(
    listener: &UnixListener,
    el_handle: &LoopHandle<Shikane<impl WlBackend>>,
) -> Result<(), Box<dyn snafu::Error>> {
    let (stream, _) = listener.accept().context(SocketAcceptCtx)?;
    let ipc: IpcStream = stream.into();
    let stream_event_source = ipc.into_event_source();

    trace!("[EventLoop] Inserting IPC client");
    el_handle
        .insert_source(stream_event_source, |_, stream, shikane| {
            // this is safe because the inner stream does not get dropped
            // also this source gets immediately removed at the end of this closure
            if let Err(err) = handle_client(unsafe { stream.get_mut() }, &mut shikane.dsm) {
                let err = error::report(err.as_ref());
                warn!("IPC error({})", err);
            }
            Ok(calloop::PostAction::Remove)
        })
        .context(InsertCtx)?;
    Ok(())
}

fn handle_client(
    ipc: &mut IpcStream,
    state: &mut Dsm<impl WlBackend>,
) -> Result<(), Box<dyn snafu::Error>> {
    let request: IpcRequest = ipc.recv()?;
    let response = delegate_command(request, state);
    ipc.send(&response)?;
    Ok(())
}

fn delegate_command(command: IpcRequest, state: &mut Dsm<impl WlBackend>) -> IpcResponse {
    match command {
        IpcRequest::CurrentHeads => req_current_heads(state),
        IpcRequest::CurrentState => req_current_variant(state),
        IpcRequest::MatchReports => req_match_reports(state),
        IpcRequest::ReloadConfig(path) => req_reload_config(state, path),
        IpcRequest::SwitchProfile(pname) => req_switch_profile(state, pname),
    }
}

fn req_current_heads(state: &Dsm<impl WlBackend>) -> IpcResponse {
    match state.backend.export_heads() {
        Some(heads) => IpcResponse::CurrentHeads(heads),
        None => IpcResponse::Error("no heads available".to_string()),
    }
}

fn req_current_variant(state: &Dsm<impl WlBackend>) -> IpcResponse {
    match state.state() {
        s @ DSMState::VariantApplied(v) | s @ DSMState::VariantInProgress(v) => {
            IpcResponse::Generic(format!("{s}: {}", v.profile.name))
        }
        s => IpcResponse::Generic(format!("{s}")),
    }
}

fn req_match_reports(state: &Dsm<impl WlBackend>) -> IpcResponse {
    let reports = state.pm.reports().clone();
    IpcResponse::MatchReports(reports)
}

fn req_reload_config(state: &mut Dsm<impl WlBackend>, path: Option<PathBuf>) -> IpcResponse {
    if let Err(err) = state.settings.reload_config(path) {
        return IpcResponse::Error(error::report(err.as_ref()).to_string());
    }
    state.pm = ProfileManager::new(state.settings.profiles.clone());
    state.simulate_change();
    IpcResponse::Success
}

fn req_switch_profile(state: &mut Dsm<impl WlBackend>, profile_name: String) -> IpcResponse {
    let restriction: Restriction = SearchPattern::Fulltext(profile_name.clone()).into();
    if !state.pm.test_restriction(&restriction) {
        return IpcResponse::Error(format!("No matching profile found {:?}", profile_name));
    }
    state.pm.restrict(restriction);
    state.simulate_change();
    IpcResponse::Success
}
