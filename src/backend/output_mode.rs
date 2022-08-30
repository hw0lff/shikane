use super::{Data, ShikaneBackend};

use wayland_client::{Connection, Dispatch, Proxy, QueueHandle};
use wayland_protocols_wlr::output_management::v1::client::zwlr_output_mode_v1::Event as ZwlrOutputModeEvent;
use wayland_protocols_wlr::output_management::v1::client::zwlr_output_mode_v1::ZwlrOutputModeV1;

#[allow(unused_imports)]
use log::{debug, error, info, trace, warn};

#[derive(Default, Debug)]
pub(crate) struct OutputMode {
    pub(crate) width: i32,
    pub(crate) height: i32,
    pub(crate) refresh: i32,
    pub(crate) preferred: bool,
}

impl Dispatch<ZwlrOutputModeV1, Data> for ShikaneBackend {
    fn event(
        state: &mut Self,
        proxy: &ZwlrOutputModeV1,
        event: <ZwlrOutputModeV1 as Proxy>::Event,
        _: &Data,
        _: &Connection,
        _qhandle: &QueueHandle<Self>,
    ) {
        // Initialize the OutputMode and ensure it is in the HashMap
        let mut mode;
        match state.output_modes.get_mut(&proxy.id()) {
            Some(m) => mode = m,
            None => {
                state.output_modes.insert(proxy.id(), OutputMode::default());
                mode = state.output_modes.get_mut(&proxy.id()).unwrap();
            }
        };

        // Update the properties of a mode
        match event {
            ZwlrOutputModeEvent::Size { width, height } => {
                trace!(
                    "[OutputMode::Event::Size] width: {:?}, height: {:?}",
                    width,
                    height
                );
                (mode.width, mode.height) = (width, height)
            }
            ZwlrOutputModeEvent::Refresh { refresh } => {
                trace!("[OutputMode::Event::Refresh] {:?}", refresh);
                mode.refresh = refresh
            }
            ZwlrOutputModeEvent::Preferred => {
                // I'm not sure if the server can change the preferation of a mode.
                trace!("[OutputMode::Event::Preferred]");
                mode.preferred = true
            }
            ZwlrOutputModeEvent::Finished => match state.remove_mode(proxy.id()) {
                Ok(_) => {}
                Err(err) => {
                    error!("{:?}", err);
                }
            },
            _ => warn!("[OutputMode::Event] unknown event received: {:?}", event),
        }
    }
}
