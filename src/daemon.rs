use crate::args::ShikaneArgs;
use crate::backend::ShikaneBackend;
use crate::config::ShikaneConfig;
use crate::error::ShikaneError;
use crate::state::ShikaneState;

use calloop::{channel, EventLoop};
use clap::Parser;

#[allow(unused_imports)]
use log::{debug, error, info, trace, warn};

pub fn daemon() {
    if let Err(err) = run() {
        error!("{}", err)
    }
}

fn run() -> Result<(), ShikaneError> {
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
