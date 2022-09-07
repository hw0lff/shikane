use crate::backend::StateInput;

use super::{Data, ShikaneBackend};

use wayland_client::{Connection, Dispatch, Proxy, QueueHandle};
use wayland_protocols_wlr::output_management::v1::client::zwlr_output_configuration_v1::Event as OutputConfigurationEvent;
use wayland_protocols_wlr::output_management::v1::client::zwlr_output_configuration_v1::ZwlrOutputConfigurationV1;

#[allow(unused_imports)]
use log::{debug, error, info, trace, warn};

impl Dispatch<ZwlrOutputConfigurationV1, Data> for ShikaneBackend {
    fn event(
        state: &mut Self,
        _proxy: &ZwlrOutputConfigurationV1,
        event: <ZwlrOutputConfigurationV1 as Proxy>::Event,
        _data: &Data,
        _conn: &Connection,
        _qhandle: &QueueHandle<Self>,
    ) {
        trace!("[Event] {:?}", event);
        match event {
            OutputConfigurationEvent::Succeeded => {
                state.send(StateInput::OutputConfigurationSucceeded);
            }
            OutputConfigurationEvent::Failed => {
                state.send(StateInput::OutputConfigurationFailed);
            }
            OutputConfigurationEvent::Cancelled => {
                state.send(StateInput::OutputConfigurationCancelled);
            }
            _ => warn!("[Event] unknown event received: {:?}", event),
        };
    }
}
