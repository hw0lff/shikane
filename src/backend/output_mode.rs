use super::{Data, ShikaneBackend};

use wayland_client::{Connection, Dispatch, Proxy, QueueHandle};
use wayland_protocols_wlr::output_management::v1::client::zwlr_output_mode_v1::Event as ZwlrOutputModeEvent;
use wayland_protocols_wlr::output_management::v1::client::zwlr_output_mode_v1::ZwlrOutputModeV1;

#[allow(unused_imports)]
use log::{debug, error, info, trace, warn};

#[derive(Default, Debug)]
pub struct OutputMode {
    pub width: i32,
    pub height: i32,
    pub refresh: i32,
    pub preferred: bool,
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
                trace!("[Event::Size] width: {:?}, height: {:?}", width, height);
                (mode.width, mode.height) = (width, height)
            }
            ZwlrOutputModeEvent::Refresh { refresh } => {
                trace!("[Event::Refresh] {:?}", refresh);
                mode.refresh = refresh
            }
            ZwlrOutputModeEvent::Preferred => {
                // I'm not sure if the server can change the preferation of a mode.
                trace!("[Event::Preferred]");
                mode.preferred = true
            }
            ZwlrOutputModeEvent::Finished => {
                trace!("[Event::Finished]");
                proxy.release();

                match state.remove_mode(proxy.id()) {
                    Ok(_) => {}
                    Err(err) => {
                        error!("{:?}", err);
                    }
                }
            }
            _ => warn!("[Event] unknown event received: {:?}", event),
        }
    }
}

impl OutputMode {
    /// Returns [`true`] if the supplied parameters align with the parameters of the mode.
    /// `width` and `height` are in pixel, `refresh` is in Hz.
    pub fn matches2(&self, width: i32, height: i32, refresh: i32) -> bool {
        // | refresh - monitor.refresh | * 100
        // ----------------------------------- < epsilon
        //               refresh
        self.width == width && self.height == height && {
            const EPSILON: f32 = 0.2; // maximum relative difference in %
            let refresh: i32 = refresh * 1000; // convert Hz to mHZ
            trace!(
                "refresh: {}mHz, monitor.refresh {}mHz",
                refresh,
                self.refresh
            );
            let diff: i32 = refresh.abs_diff(self.refresh) as i32; // difference in mHz

            // times 100 to calculate in %
            let p: f32 = (diff * 100) as f32 / refresh as f32; // relative difference in %
            trace!("diff: {diff}mHz, ratio(diff,refresh): {p}%");
            p < EPSILON
        }
    }
}
