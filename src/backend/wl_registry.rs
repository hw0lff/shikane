use super::{Data, ShikaneBackend};

use wayland_client::{protocol::wl_registry, Connection, Dispatch, QueueHandle};
use wayland_protocols_wlr::output_management::v1::client::zwlr_output_manager_v1::ZwlrOutputManagerV1;

#[allow(unused_imports)]
use log::{debug, error, info, trace, warn};

impl Dispatch<wl_registry::WlRegistry, Data> for ShikaneBackend {
    fn event(
        state: &mut Self,
        registry: &wl_registry::WlRegistry,
        event: wl_registry::Event,
        _: &Data,
        _: &Connection,
        qhandle: &QueueHandle<Self>,
    ) {
        if let wl_registry::Event::Global {
            name,
            interface,
            version,
        } = event
        {
            match &interface[..] {
                "zwlr_output_manager_v1" => {
                    let wlr_output_manager = registry
                        .bind::<ZwlrOutputManagerV1, _, _>(name, version, qhandle, Data::default())
                        .unwrap();
                    state.wlr_output_manager = Some(wlr_output_manager);
                    trace!("[WlRegistry::bind] [{}] {} (v{})", name, interface, version);
                }
                /*
                "zwlr_output_power_manager_v1" => {
                    let wlr_output_power_manager = registry
                                .bind::<ZwlrOutputPowerManagerV1, _, _>(name, version, qhandle, ())
                                .unwrap();
                                                state.wlr_output_power_manager = Some(wlr_output_power_manager);
                            trace!("[{}] {} (v{})", name, interface, version);
                }
                */
                _ => {}
            }
        }
    }
}
