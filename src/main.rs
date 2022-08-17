use std::collections::HashMap;

use wayland_client::backend::ObjectId;
use wayland_client::event_created_child;
use wayland_client::protocol::wl_output::Transform;
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
use thiserror::Error;

#[derive(Default, Debug)]
struct ShikaneState {
    done: bool,
    output_manager_serial: u32,
    wlr_output_manager: Option<ZwlrOutputManagerV1>,
    /// A Mapping from ZwlrOutputHeadV1-Ids to OutputHeads
    output_heads: HashMap<ObjectId, OutputHead>,
    /// A Mapping from ZwlrOutputModeV1-Ids to OutputModes
    output_modes: HashMap<ObjectId, OutputMode>,
    /// A Mapping from ZwlrOutputModeV1-Ids to ZwlrOutputHeadV1-Ids
    ///
    /// The ZwlrOutputModeV1 from the key-field belongs to the ZwlrOutputHeadV1 in the value-field
    mode_id_head_id: HashMap<ObjectId, ObjectId>,
}

#[derive(Error, Debug)]
enum ShikaneError {
    #[error("Unable to release resources associated with destroyed mode")]
    ReleaseOutputMode,
}

#[derive(Default, Debug)]
struct OutputHead {
    name: String,
    description: String,
    physical_width: i32,
    physical_height: i32,
    modes: Vec<ObjectId>,
    enabled: bool,
    current_mode: Option<ObjectId>,
    position_x: i32,
    position_y: i32,
    transform: Option<Transform>,
    scale: f64,
    make: String,
    model: String,
    serial_number: String,
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

impl ShikaneState {
    /// After received the Finished event for a ZwlrOutputModeV1 the mode must not be used anymore.
    /// This function removes all occurences of the provided ObjectId of the mode in ShikaneState.
    fn remove_mode(&mut self, id: ObjectId) -> Result<(), ShikaneError> {
        // the id of the head the mode belongs to
        let head_id = self
            .mode_id_head_id
            .remove(&id)
            .ok_or(ShikaneError::ReleaseOutputMode)?;
        let head = self
            .output_heads
            .get_mut(&head_id)
            .ok_or(ShikaneError::ReleaseOutputMode)?;
        if let Some(c_mode_id) = &head.current_mode {
            if *c_mode_id == id {
                head.current_mode = None;
            }
        }
        head.modes.retain(|e| *e != id);
        Ok(())
    }
}

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
                trace!("[OutputManager::Event::Head] head.id = {:?}", head.id());
                state.output_heads.insert(head.id(), OutputHead::default());
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
        state: &mut Self,
        proxy: &ZwlrOutputHeadV1,
        event: <ZwlrOutputHeadV1 as Proxy>::Event,
        _: &Data,
        _: &Connection,
        _qhandle: &QueueHandle<Self>,
    ) {
        // Initialize the OutputHead and ensure it is in the HashMap
        let mut head;
        match state.output_heads.get_mut(&proxy.id()) {
            Some(m) => head = m,
            None => {
                state.output_heads.insert(proxy.id(), OutputHead::default());
                head = state.output_heads.get_mut(&proxy.id()).unwrap();
            }
        };

        // Update the properties of a head
        match event {
            ZwlrOutputHeadEvent::Name { name } => head.name = name,
            ZwlrOutputHeadEvent::Description { description } => head.description = description,
            ZwlrOutputHeadEvent::PhysicalSize { width, height } => {
                (head.physical_width, head.physical_height) = (width, height)
            }
            ZwlrOutputHeadEvent::Mode { mode } => {
                trace!("[OutputHead::Event::Mode] mode.id = {:?}", mode.id());
                state.mode_id_head_id.insert(mode.id(), proxy.id());
                head.modes.push(mode.id());
            }
            ZwlrOutputHeadEvent::Enabled { enabled } => head.enabled = !matches!(enabled, 0),
            ZwlrOutputHeadEvent::CurrentMode { mode } => {
                trace!("[OutputHead::Event::CurrentMode] mode.id = {:?}", mode.id());
                head.current_mode = Some(mode.id())
            }
            ZwlrOutputHeadEvent::Position { x, y } => (head.position_x, head.position_y) = (x, y),
            ZwlrOutputHeadEvent::Transform { transform } => {
                head.transform = match transform.into_result() {
                    Ok(transform) => Some(transform),
                    Err(err) => {
                        warn!(
                        "[OutputHead::Event::Transform] The stored value does not match one defined by the protocol file: {:?}",
                        err
                    );
                        None
                    }
                }
            }
            ZwlrOutputHeadEvent::Scale { scale } => head.scale = scale,

            ZwlrOutputHeadEvent::Finished => {
                state.output_heads.remove(&proxy.id());
            }
            ZwlrOutputHeadEvent::Make { make } => head.make = make,
            ZwlrOutputHeadEvent::Model { model } => head.model = model,

            ZwlrOutputHeadEvent::SerialNumber { serial_number } => {
                head.serial_number = serial_number
            }
            _ => {
                warn!("[OutputHead::Event] Unknown event {:?}", event)
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
        _: &Data,
        _: &Connection,
        _qhandle: &QueueHandle<Self>,
    ) {
        trace!("[OutputMode::Event] {:?}", event);

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
                (mode.width, mode.height) = (width, height)
            }
            ZwlrOutputModeEvent::Refresh { refresh } => mode.refresh = refresh,
            ZwlrOutputModeEvent::Preferred => {
                // I'm not sure if the server can change the preferation of a mode.
                mode.preferred = true
            }
            ZwlrOutputModeEvent::Finished => match state.remove_mode(proxy.id()) {
                Ok(_) => {}
                Err(err) => {
                    error!("{:?}", err);
                }
            },
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
    env_logger::Builder::from_env("SHIKANE_LOG")
        .format_timestamp(None)
        .init();

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

    let head_id = state
        .output_heads
        .keys()
        .find(|key| {
            let sh_head = state.output_heads.get(*key).unwrap();
            sh_head.name != "eDP-1"
        })
        .unwrap();
    let output_head = state.output_heads.get(head_id).unwrap();
    debug!("{:#?}", output_head);

    let c_mode_id = output_head.current_mode.as_ref().unwrap();
    let c_mode = state.output_modes.get(c_mode_id).unwrap();
    debug!("{:#?}", c_mode);

    let head = ZwlrOutputHeadV1::from_id(&conn, head_id.clone()).unwrap();
    let opch = opc.enable_head(&head, &qh, Data::default()).unwrap();
    opch.set_scale(1.0);
    opc.apply();

    event_queue.blocking_dispatch(&mut state).unwrap();
}
