use super::output_head::OutputHead;
use super::{Data, ShikaneBackend};

use wayland_client::event_created_child;
use wayland_client::{Connection, Dispatch, Proxy, QueueHandle};
use wayland_protocols_wlr::output_management::v1::client::zwlr_output_head_v1::ZwlrOutputHeadV1;
use wayland_protocols_wlr::output_management::v1::client::zwlr_output_manager_v1::Event as ZwlrOutputManagerEvent;
use wayland_protocols_wlr::output_management::v1::client::zwlr_output_manager_v1::ZwlrOutputManagerV1;

#[allow(unused_imports)]
use log::{debug, error, info, trace, warn};

impl Dispatch<ZwlrOutputManagerV1, Data> for ShikaneBackend {
    fn event(
        state: &mut Self,
        _proxy: &ZwlrOutputManagerV1,
        event: <ZwlrOutputManagerV1 as Proxy>::Event,
        _data: &Data,
        _conn: &Connection,
        _qhandle: &QueueHandle<Self>,
    ) {
        match event {
            ZwlrOutputManagerEvent::Head { head } => {
                trace!("[Event::Head] id: {:?}", head.id());
                state.output_heads.insert(head.id(), OutputHead::default());
            }
            ZwlrOutputManagerEvent::Done { serial } => {
                trace!("[Event::Done] serial: {}", serial);
                state.output_manager_serial = serial;
                state.done = true;
            }
            ZwlrOutputManagerEvent::Finished => {
                trace!("[Event::Finished]")
            }
            _ => warn!("[Event] unknown event received: {:?}", event),
        }
    }

    event_created_child!(ShikaneBackend, ZwlrOutputManagerV1, [
        0 => (ZwlrOutputHeadV1, Data::default()),
    ]);
}
