use std::collections::HashMap;

use wayland_client::backend::ObjectId;
use wayland_client::event_created_child;
use wayland_client::{protocol::wl_registry, Connection, Dispatch, Proxy, QueueHandle};
use wayland_protocols_wlr::output_management::v1::client::zwlr_output_configuration_head_v1::ZwlrOutputConfigurationHeadV1;
use wayland_protocols_wlr::output_management::v1::client::zwlr_output_configuration_v1::ZwlrOutputConfigurationV1;
use wayland_protocols_wlr::output_management::v1::client::zwlr_output_head_v1::Event as ZwlrOutputHeadEvent;
use wayland_protocols_wlr::output_management::v1::client::zwlr_output_head_v1::ZwlrOutputHeadV1;
use wayland_protocols_wlr::output_management::v1::client::zwlr_output_manager_v1;
use wayland_protocols_wlr::output_management::v1::client::zwlr_output_manager_v1::ZwlrOutputManagerV1;
use wayland_protocols_wlr::output_management::v1::client::zwlr_output_mode_v1::Event as ZwlrOutputModeEvent;
use wayland_protocols_wlr::output_management::v1::client::zwlr_output_mode_v1::ZwlrOutputModeV1;

#[allow(unused_imports)]
use log::{debug, error, info, trace, warn};

#[derive(Default, Debug)]
struct ShikaneState {
    done: bool,
    output_manager_serial: u32,
    wlr_output_manager: Option<ZwlrOutputManagerV1>,
    wlr_output_heads: Vec<ZwlrOutputHeadV1>,
    /// Maps OutputMode IDs to OutputModes
    wlr_output_modes: HashMap<ObjectId, OutputMode>,
}

#[derive(Default, Debug)]
struct OutputMode {
    width: i32,
    height: i32,
    refresh: i32,
    preferred: bool,
}

#[derive(Default, Debug)]
struct Data;

impl Dispatch<wl_registry::WlRegistry, Data> for ShikaneState {
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

impl Dispatch<ZwlrOutputManagerV1, Data> for ShikaneState {
    fn event(
        state: &mut Self,
        _proxy: &ZwlrOutputManagerV1,
        event: <ZwlrOutputManagerV1 as Proxy>::Event,
        _data: &Data,
        _conn: &Connection,
        _qhandle: &QueueHandle<Self>,
    ) {
        match event {
            zwlr_output_manager_v1::Event::Head { head } => {
                trace!("[OutputManager::Event::Head] head = {:?}", head);
                state.wlr_output_heads.push(head);
            }
            zwlr_output_manager_v1::Event::Done { serial } => {
                trace!("[OutputManager::Event::Done] serial = {}", serial);
                state.output_manager_serial = serial;
                state.done = true;
            }
            zwlr_output_manager_v1::Event::Finished => {
                trace!("[OutputManager::Event::Finished]")
            }
            _ => warn!("[OutputManager::Event] unknown Event received: {:?}", event),
        }
    }

    event_created_child!(ShikaneState, ZwlrOutputManagerV1, [
        0 => (ZwlrOutputHeadV1, Data::default()),
    ]);
}

impl Dispatch<ZwlrOutputHeadV1, Data> for ShikaneState {
    fn event(
        _state: &mut Self,
        _proxy: &ZwlrOutputHeadV1,
        event: <ZwlrOutputHeadV1 as Proxy>::Event,
        _data: &Data,
        _conn: &Connection,
        _qhandle: &QueueHandle<Self>,
    ) {
        match event {
            ZwlrOutputHeadEvent::Mode { mode } => {
                trace!("[OutputHead::Event::Mode] mode.id = {:?}", mode.id());
            }
            ZwlrOutputHeadEvent::CurrentMode { mode } => {
                trace!("[OutputHead::Event::CurrentMode] mode.id = {:?}", mode.id());
            }
            _ => {
                trace!("[OutputHead::Event] {:?}", event)
            }
        }
    }

    event_created_child!(ShikaneState, ZwlrOutputManagerV1, [
        3 => (ZwlrOutputModeV1, Data::default()),
    ]);
}

impl Dispatch<ZwlrOutputModeV1, Data> for ShikaneState {
    fn event(
        state: &mut Self,
        proxy: &ZwlrOutputModeV1,
        event: <ZwlrOutputModeV1 as Proxy>::Event,
        _data: &Data,
        _conn: &Connection,
        _qhandle: &QueueHandle<Self>,
    ) {
        trace!("[OutputMode::Event] {:?}", event);

        let mut mode;
        match state.wlr_output_modes.get_mut(&proxy.id()) {
            Some(m) => mode = m,
            None => {
                state
                    .wlr_output_modes
                    .insert(proxy.id(), OutputMode::default());
                mode = state.wlr_output_modes.get_mut(&proxy.id()).unwrap();
            }
        };

        match event {
            ZwlrOutputModeEvent::Size { width, height } => {
                (mode.width, mode.height) = (width, height)
            }
            ZwlrOutputModeEvent::Refresh { refresh } => mode.refresh = refresh,
            ZwlrOutputModeEvent::Preferred => mode.preferred = true,
            ZwlrOutputModeEvent::Finished => {
                state.wlr_output_modes.remove(&proxy.id()).unwrap();
            }
            _ => warn!("[OutputMode::Event] unknown Event received: {:?}", event),
        }
    }
}

impl Dispatch<ZwlrOutputConfigurationHeadV1, Data> for ShikaneState {
    fn event(
        _state: &mut Self,
        _proxy: &ZwlrOutputConfigurationHeadV1,
        event: <ZwlrOutputConfigurationHeadV1 as Proxy>::Event,
        _data: &Data,
        _conn: &Connection,
        _qhandle: &QueueHandle<Self>,
    ) {
        trace!("[OutputConfiguration::Event] {:?}", event);
    }
}

impl Dispatch<ZwlrOutputConfigurationV1, Data> for ShikaneState {
    fn event(
        _state: &mut Self,
        _proxy: &ZwlrOutputConfigurationV1,
        event: <ZwlrOutputConfigurationV1 as Proxy>::Event,
        _data: &Data,
        _conn: &Connection,
        _qhandle: &QueueHandle<Self>,
    ) {
        trace!("[OutputConfiguration::Event] {:?}", event);
    }
}

fn main() {
    // env_logger::init();
    env_logger::builder().format_timestamp(None).init();

    let conn = Connection::connect_to_env().unwrap();

    let display = conn.display();

    let mut event_queue = conn.new_event_queue();
    let qh = event_queue.handle();

    let _registry = display.get_registry(&qh, Data::default()).unwrap();

    let mut state = ShikaneState::default();

    event_queue.roundtrip(&mut state).unwrap();

    while !state.done {
        event_queue.blocking_dispatch(&mut state).unwrap();
    }

    let opc = state
        .wlr_output_manager
        .as_ref()
        .unwrap()
        .create_configuration(state.output_manager_serial, &qh, Data::default())
        .unwrap();

    opc.enable_head(
        state.wlr_output_heads.first().unwrap(),
        &qh,
        Data::default(),
    )
    .unwrap()
    .set_scale(1.0);

    opc.apply();

    event_queue.blocking_dispatch(&mut state).unwrap();
}
