mod args;
mod backend;
mod config;
mod error;
mod exec;
mod hk;
mod profile;
mod state;
use backend::ShikaneBackend;
use clap::Parser;
use error::ShikaneError;
use state::ShikaneState;

use calloop::{channel, EventLoop};

#[allow(unused_imports)]
use log::{debug, error, info, trace, warn};

use crate::{args::ShikaneArgs, config::ShikaneConfig};

fn main() {
    match run() {
        Ok(_) => {}
        Err(err) => error!("{}", err),
    }
}

fn run() -> Result<(), ShikaneError> {
    env_logger::Builder::from_env("SHIKANE_LOG")
        .format_timestamp(None)
        .init();
    let args = ShikaneArgs::parse();
    let config = ShikaneConfig::parse(args.config.clone())?;

    let mut event_loop: EventLoop<ShikaneState> = EventLoop::try_new()?;
    let (sender, channel) = channel::channel();
    let (backend, wl_source) = ShikaneBackend::connect(sender)?;
    let mut state = ShikaneState::new(args, backend, config, event_loop.get_signal());
    let el_handle = event_loop.handle();

    el_handle.insert_source(channel, |event, _, state| match event {
        channel::Event::Msg(m) => state.advance(m),
        channel::Event::Closed => todo!(),
    })?;

    el_handle.insert_source(wl_source, |_, event_queue, state| {
        state.backend.callback(event_queue)
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
