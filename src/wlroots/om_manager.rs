#[allow(unused_imports)]
use log::{debug, error, info, trace, warn};
use wayland_client::event_created_child;
use wayland_client::{Connection, Dispatch, Proxy, QueueHandle};
use wayland_protocols_wlr::output_management::v1::client::zwlr_output_head_v1::ZwlrOutputHeadV1;
use wayland_protocols_wlr::output_management::v1::client::zwlr_output_manager_v1::Event as ZwlrOutputManagerEvent;
use wayland_protocols_wlr::output_management::v1::client::zwlr_output_manager_v1::ZwlrOutputManagerV1;
use wayland_protocols_wlr::output_management::v1::client::zwlr_output_manager_v1::EVT_HEAD_OPCODE;

use crate::wl_backend::WlBackendEvent;

use super::WlrootsBackend;

impl Dispatch<ZwlrOutputManagerV1, ()> for WlrootsBackend {
    fn event(
        backend: &mut Self,
        _: &ZwlrOutputManagerV1,
        event: <ZwlrOutputManagerV1 as Proxy>::Event,
        _: &(),
        _: &Connection,
        _: &QueueHandle<Self>,
    ) {
        match event {
            ZwlrOutputManagerEvent::Head { head } => {
                trace!("[Event::Head] id: {:?}", head.id());
                backend.wl_store.insert_head(head)
            }
            ZwlrOutputManagerEvent::Done { serial } => {
                trace!("[Event::Done] serial: {}", serial);
                backend.wlr_output_manager_serial = serial;
                backend.queue_event(WlBackendEvent::AtomicChangeDone);
            }
            ZwlrOutputManagerEvent::Finished => {
                trace!("[Event::Finished]");
                backend.wlr_output_manager_serial = 0;
                backend.queue_event(WlBackendEvent::NeededResourceFinished);
            }
            unknown => warn!("[Event] Unknown event received: {unknown:?}"),
        }
    }

    event_created_child!(WlrootsBackend, ZwlrOutputManagerV1, [
        EVT_HEAD_OPCODE=> (ZwlrOutputHeadV1, ()),
    ]);
}
