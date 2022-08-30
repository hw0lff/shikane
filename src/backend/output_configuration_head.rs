use super::{Data, ShikaneBackend};

use wayland_client::{Connection, Dispatch, Proxy, QueueHandle};
use wayland_protocols_wlr::output_management::v1::client::zwlr_output_configuration_head_v1::ZwlrOutputConfigurationHeadV1;

#[allow(unused_imports)]
use log::{debug, error, info, trace, warn};

impl Dispatch<ZwlrOutputConfigurationHeadV1, Data> for ShikaneBackend {
    fn event(
        _state: &mut Self,
        _proxy: &ZwlrOutputConfigurationHeadV1,
        event: <ZwlrOutputConfigurationHeadV1 as Proxy>::Event,
        _data: &Data,
        _conn: &Connection,
        _qhandle: &QueueHandle<Self>,
    ) {
        trace!("[OutputConfigurationHead::Event] {:?}", event);
    }
}
