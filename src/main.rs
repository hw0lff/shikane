mod backend;
mod state;
use backend::Data;
use state::ShikaneState;

use wayland_client::Connection;
use wayland_client::Proxy;
use wayland_protocols_wlr::output_management::v1::client::zwlr_output_head_v1::ZwlrOutputHeadV1;

#[allow(unused_imports)]
use log::{debug, error, info, trace, warn};

fn main() {
    // env_logger::init();
    env_logger::Builder::from_env("SHIKANE_LOG")
        .format_timestamp(None)
        .init();

    let conn = Connection::connect_to_env().unwrap();

    let display = conn.display();

    let mut event_queue = conn.new_event_queue();
    let qh = event_queue.handle();

    let _registry = display.get_registry(&qh, Data::default()).unwrap();

    let mut state = ShikaneState::default();

    event_queue.roundtrip(&mut state.backend).unwrap();

    while !state.backend.done {
        event_queue.blocking_dispatch(&mut state.backend).unwrap();
    }

    let opc = state
        .backend
        .wlr_output_manager
        .as_ref()
        .unwrap()
        .create_configuration(state.backend.output_manager_serial, &qh, Data::default())
        .unwrap();

    let head_id = state
        .backend
        .output_heads
        .keys()
        .find(|key| {
            let sh_head = state.backend.output_heads.get(*key).unwrap();
            sh_head.name != "eDP-1"
        })
        .unwrap();
    let output_head = state.backend.output_heads.get(head_id).unwrap();
    debug!("{:#?}", output_head);

    let c_mode_id = output_head.current_mode.as_ref().unwrap();
    let c_mode = state.backend.output_modes.get(c_mode_id).unwrap();
    debug!("{:#?}", c_mode);

    let head = ZwlrOutputHeadV1::from_id(&conn, head_id.clone()).unwrap();
    let opch = opc.enable_head(&head, &qh, Data::default()).unwrap();
    opch.set_scale(1.0);
    opc.apply();

    event_queue.blocking_dispatch(&mut state.backend).unwrap();
}
