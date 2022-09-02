mod args;
mod backend;
mod config;
mod state;
use backend::ShikaneBackend;
use clap::Parser;
use state::ShikaneState;

use calloop::{ping::make_ping, EventLoop};

#[allow(unused_imports)]
use log::{debug, error, info, trace, warn};

use crate::{args::ShikaneArgs, config::ShikaneConfig};

fn main() {
    env_logger::Builder::from_env("SHIKANE_LOG")
        .format_timestamp(None)
        .init();
    let args = ShikaneArgs::parse();
    let config = ShikaneConfig::parse(args.config);

    let mut event_loop: EventLoop<ShikaneState> = EventLoop::try_new().unwrap();
    let (backend, wl_source) = ShikaneBackend::connect();
    let mut state = ShikaneState::new(backend, config);
    let el_handle = event_loop.handle();
    let (ping, ping_source) = make_ping().unwrap();

    el_handle
        .insert_source(ping_source, |_e, _b, state| state.configure())
        .expect("failed to insert config applier");

    el_handle
        .insert_source(wl_source, move |_, event_queue, state| {
            let callback_result = state.backend.callback(event_queue);

            if !state.first_done && state.backend.done {
                state.first_done = true;
                trace!("[Backend] pinging Config");
                ping.ping();
            }
            callback_result
        })
        .expect("failed to insert wayland source");

    // Idle timeout callback
    event_loop
        .run(std::time::Duration::from_millis(500), &mut state, |state| {
            state.idle();
        })
        .unwrap();
}
