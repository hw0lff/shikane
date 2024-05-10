#[allow(unused_imports)]
use log::{debug, error, info, trace, warn};
use wayland_client::{Connection, Dispatch, Proxy, QueueHandle};
use wayland_protocols_wlr::output_management::v1::client::zwlr_output_mode_v1::Event as ZwlrOutputModeEvent;
use wayland_protocols_wlr::output_management::v1::client::zwlr_output_mode_v1::ZwlrOutputModeV1;

use crate::error;

use super::WlrootsBackend;

impl Dispatch<ZwlrOutputModeV1, ()> for WlrootsBackend {
    fn event(
        backend: &mut Self,
        wlr_mode: &ZwlrOutputModeV1,
        event: <ZwlrOutputModeV1 as Proxy>::Event,
        _: &(),
        _: &Connection,
        _: &QueueHandle<Self>,
    ) {
        let mode = match backend.wl_store.mode_mut(wlr_mode.id()) {
            Ok(mode) => mode,
            Err(err) => {
                warn!("{}", error::report(&err));
                return;
            }
        };

        // Update the properties of a mode
        match event {
            ZwlrOutputModeEvent::Size { width, height } => {
                trace!("[Event::Size] width: {:?}, height: {:?}", width, height);
                (mode.base.width, mode.base.height) = (width, height)
            }
            ZwlrOutputModeEvent::Refresh { refresh } => {
                trace!("[Event::Refresh] {:?}", refresh);
                mode.base.refresh = refresh
            }
            ZwlrOutputModeEvent::Preferred => {
                // I'm not sure if the server can change the preferation of a mode.
                trace!("[Event::Preferred]");
                mode.base.preferred = true
            }
            ZwlrOutputModeEvent::Finished => {
                trace!("[Event::Finished]");
                // After receiving the Finished event for a `ZwlrOutputModeV1` the mode must not be used anymore.
                wlr_mode.release();
                // Thus removing the mode from the store.
                if let Err(err) = backend.wl_store.remove_mode(&wlr_mode.id()) {
                    warn!("{:?}", error::report(&err));
                }
            }
            _ => warn!("[Event] unknown event received: {:?}", event),
        }
    }
}
