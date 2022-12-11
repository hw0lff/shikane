use std::fmt::Display;

use super::{Data, ShikaneBackend};

use wayland_client::{Connection, Dispatch, Proxy, QueueHandle};
use wayland_protocols_wlr::output_management::v1::client::zwlr_output_mode_v1::Event as ZwlrOutputModeEvent;
use wayland_protocols_wlr::output_management::v1::client::zwlr_output_mode_v1::ZwlrOutputModeV1;

#[allow(unused_imports)]
use log::{debug, error, info, trace, warn};

#[derive(Debug)]
pub struct OutputMode {
    pub width: i32,
    pub height: i32,
    pub refresh: i32,
    pub preferred: bool,
    pub wlr_mode: ZwlrOutputModeV1,
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
                state
                    .output_modes
                    .insert(proxy.id(), OutputMode::new(proxy.clone()));
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
    pub fn new(wlr_mode: ZwlrOutputModeV1) -> Self {
        Self {
            width: Default::default(),
            height: Default::default(),
            refresh: Default::default(),
            preferred: Default::default(),
            wlr_mode,
        }
    }

    /// `refresh` is in Hz
    pub fn matches(&self, refresh: i32, delta: &mut i32) -> bool {
        const MAX_DELTA: i32 = 500; // maximum difference in mHz
        let refresh: i32 = refresh * 1000; // convert Hz to mHZ
        let diff: i32 = refresh.abs_diff(self.refresh) as i32; // difference in mHz
        trace!(
            "refresh: {refresh}mHz, monitor.refresh {}mHz, diff: {diff}mHz",
            self.refresh
        );

        if diff < MAX_DELTA && diff < *delta {
            *delta = diff;
            return true;
        }
        false
    }
}

impl Display for OutputMode {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}x{}@{}mHz", self.width, self.height, self.refresh)
    }
}
