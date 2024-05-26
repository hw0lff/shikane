#[allow(unused_imports)]
use log::{debug, error, info, trace, warn};
use wayland_client::{Connection, Dispatch, Proxy, QueueHandle};
use wayland_protocols_wlr::output_management::v1::client::zwlr_output_configuration_head_v1::ZwlrOutputConfigurationHeadV1;
use wayland_protocols_wlr::output_management::v1::client::zwlr_output_configuration_v1::Event as OutputConfigurationEvent;
use wayland_protocols_wlr::output_management::v1::client::zwlr_output_configuration_v1::ZwlrOutputConfigurationV1;

use crate::wl_backend::WlBackendEvent;

use super::WlrootsBackend;

impl Dispatch<ZwlrOutputConfigurationV1, ()> for WlrootsBackend {
    fn event(
        backend: &mut Self,
        wlr_configuration: &ZwlrOutputConfigurationV1,
        event: <ZwlrOutputConfigurationV1 as Proxy>::Event,
        _: &(),
        _: &Connection,
        _: &QueueHandle<Self>,
    ) {
        trace!("[Event] {:?}", event);
        wlr_configuration.destroy();
        match event {
            OutputConfigurationEvent::Succeeded => {
                backend.queue_event(WlBackendEvent::Succeeded);
            }
            OutputConfigurationEvent::Failed => {
                backend.queue_event(WlBackendEvent::Failed);
            }
            OutputConfigurationEvent::Cancelled => {
                backend.queue_event(WlBackendEvent::Cancelled);
            }
            unknown => warn!("[Event] unknown event received: {unknown:?}"),
        };
    }
}

impl Dispatch<ZwlrOutputConfigurationHeadV1, ()> for WlrootsBackend {
    fn event(
        _: &mut Self,
        _: &ZwlrOutputConfigurationHeadV1,
        event: <ZwlrOutputConfigurationHeadV1 as Proxy>::Event,
        _: &(),
        _: &Connection,
        _: &QueueHandle<Self>,
    ) {
        trace!("[Event] {:?}", event);
    }
}
