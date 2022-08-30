mod backend;
mod state;
use state::ShikaneState;

use calloop::EventLoop;

#[allow(unused_imports)]
use log::{debug, error, info, trace, warn};

fn main() {
    env_logger::Builder::from_env("SHIKANE_LOG")
        .format_timestamp(None)
        .init();


    let mut event_loop: EventLoop<ShikaneState> = EventLoop::try_new().unwrap();
    let mut state = ShikaneState::default();
    let el_handle = event_loop.handle();
    let wayland_source = backend::connect();




    el_handle
        .insert_source(wayland_source, |_, event_queue, state| {


            state.backend.callback(event_queue)
        })
        .expect("failed to insert wayland source");

    // Idle timeout callback
    event_loop
        .run(
            std::time::Duration::from_millis(500),
            &mut state,
            |_state| {
                trace!("[Main] processing between polling");
            },
        )
        .unwrap();
}
