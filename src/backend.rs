use std::collections::HashMap;

use smithay_client_toolkit::event_loop::WaylandSource;
use wayland_client::backend::ObjectId;
use wayland_client::protocol::wl_output::Transform;
use wayland_client::{event_created_child, DispatchError, EventQueue};
use wayland_client::{protocol::wl_registry, Connection, Dispatch, Proxy, QueueHandle};
use wayland_protocols_wlr::output_management::v1::client::zwlr_output_configuration_head_v1::ZwlrOutputConfigurationHeadV1;
use wayland_protocols_wlr::output_management::v1::client::zwlr_output_configuration_v1::ZwlrOutputConfigurationV1;
use wayland_protocols_wlr::output_management::v1::client::zwlr_output_head_v1::Event as ZwlrOutputHeadEvent;
use wayland_protocols_wlr::output_management::v1::client::zwlr_output_head_v1::ZwlrOutputHeadV1;
use wayland_protocols_wlr::output_management::v1::client::zwlr_output_manager_v1::Event as ZwlrOutputManagerEvent;
use wayland_protocols_wlr::output_management::v1::client::zwlr_output_manager_v1::ZwlrOutputManagerV1;
use wayland_protocols_wlr::output_management::v1::client::zwlr_output_mode_v1::Event as ZwlrOutputModeEvent;
use wayland_protocols_wlr::output_management::v1::client::zwlr_output_mode_v1::ZwlrOutputModeV1;

#[allow(unused_imports)]
use log::{debug, error, info, trace, warn};
use thiserror::Error;

#[derive(Default, Debug)]
pub(crate) struct ShikaneBackend {
    pub(crate) done: bool,
    pub(crate) output_manager_serial: u32,
    pub(crate) wlr_output_manager: Option<ZwlrOutputManagerV1>,
    /// A Mapping from ZwlrOutputHeadV1-Ids to OutputHeads
    pub(crate) output_heads: HashMap<ObjectId, OutputHead>,
    /// A Mapping from ZwlrOutputModeV1-Ids to OutputModes
    pub(crate) output_modes: HashMap<ObjectId, OutputMode>,
    /// A Mapping from ZwlrOutputModeV1-Ids to ZwlrOutputHeadV1-Ids
    ///
    /// The ZwlrOutputModeV1 from the key-field belongs to the ZwlrOutputHeadV1 in the value-field
    pub(crate) mode_id_head_id: HashMap<ObjectId, ObjectId>,
}

#[derive(Error, Debug)]
enum ShikaneError {
    #[error("Unable to release resources associated with destroyed mode")]
    ReleaseOutputMode,
}

#[derive(Default, Debug)]
pub(crate) struct OutputHead {
    pub(crate) name: String,
    pub(crate) description: String,
    pub(crate) physical_width: i32,
    pub(crate) physical_height: i32,
    pub(crate) modes: Vec<ObjectId>,
    pub(crate) enabled: bool,
    pub(crate) current_mode: Option<ObjectId>,
    pub(crate) position_x: i32,
    pub(crate) position_y: i32,
    pub(crate) transform: Option<Transform>,
    pub(crate) scale: f64,
    pub(crate) make: String,
    pub(crate) model: String,
    pub(crate) serial_number: String,
}

#[derive(Default, Debug)]
pub(crate) struct OutputMode {
    pub(crate) width: i32,
    pub(crate) height: i32,
    pub(crate) refresh: i32,
    pub(crate) preferred: bool,
}

#[derive(Default, Debug)]
pub(crate) struct Data;

impl ShikaneBackend {
    pub(crate) fn callback(
        &mut self,
        event_queue: &mut EventQueue<ShikaneBackend>,
    ) -> Result<usize, DispatchError> {
        let dispatch_result = event_queue.dispatch_pending(self);
        trace!("[Dispatch::Result] {:?}", dispatch_result);
        dispatch_result
    }
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

impl Dispatch<ZwlrOutputManagerV1, Data> for ShikaneBackend {
    fn event(
        state: &mut Self,
        _proxy: &ZwlrOutputManagerV1,
        event: <ZwlrOutputManagerV1 as Proxy>::Event,
        _data: &Data,
        _conn: &Connection,
        _qhandle: &QueueHandle<Self>,
    ) {
        match event {
            ZwlrOutputManagerEvent::Head { head } => {
                trace!("[OutputManager::Event::Head] id: {:?}", head.id());
                state.output_heads.insert(head.id(), OutputHead::default());
            }
            ZwlrOutputManagerEvent::Done { serial } => {
                trace!("[OutputManager::Event::Done] serial: {}", serial);
                state.output_manager_serial = serial;
                state.done = true;
            }
            ZwlrOutputManagerEvent::Finished => {
                trace!("[OutputManager::Event::Finished]")
            }
            _ => warn!("[OutputManager::Event] unknown Event received: {:?}", event),
        }
    }

    event_created_child!(ShikaneBackend, ZwlrOutputManagerV1, [
        0 => (ZwlrOutputHeadV1, Data::default()),
    ]);
}

impl Dispatch<ZwlrOutputHeadV1, Data> for ShikaneBackend {
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
            ZwlrOutputHeadEvent::Name { name } => {
                trace!("[OutputHead::Event::Name] {:?}", name);
                head.name = name
            }
            ZwlrOutputHeadEvent::Description { description } => {
                trace!("[OutputHead::Event::Description] {:?}", description);
                head.description = description
            }
            ZwlrOutputHeadEvent::PhysicalSize { width, height } => {
                trace!(
                    "[OutputHead::Event::PhysicalSize] width: {:?}, height: {:?}",
                    width,
                    height
                );
                (head.physical_width, head.physical_height) = (width, height)
            }
            ZwlrOutputHeadEvent::Mode { mode } => {
                trace!("[OutputHead::Event::Mode] id: {:?}", mode.id());
                state.mode_id_head_id.insert(mode.id(), proxy.id());
                head.modes.push(mode.id());
            }
            ZwlrOutputHeadEvent::Enabled { enabled } => {
                trace!("[OutputHead::Event::Enabled]");
                head.enabled = !matches!(enabled, 0)
            }
            ZwlrOutputHeadEvent::CurrentMode { mode } => {
                trace!("[OutputHead::Event::CurrentMode] id: {:?}", mode.id());
                head.current_mode = Some(mode.id())
            }
            ZwlrOutputHeadEvent::Position { x, y } => {
                trace!("[OutputHead::Event::Position] x: {:?}, y: {:?}", x, y);
                (head.position_x, head.position_y) = (x, y)
            }
            ZwlrOutputHeadEvent::Transform { transform } => {
                head.transform = match transform.into_result() {
                    Ok(transform) => {
                        trace!("[OutputHead::Event::Transform] {:?}", transform);
                        Some(transform)
                    }
                    Err(err) => {
                        warn!(
                        "[OutputHead::Event::Transform] The stored value does not match one defined by the protocol file: {:?}",
                        err
                    );
                        None
                    }
                }
            }
            ZwlrOutputHeadEvent::Scale { scale } => {
                trace!("[OutputHead::Event::Scale] {:?}", scale);
                head.scale = scale
            }
            ZwlrOutputHeadEvent::Finished => {
                trace!("[OutputHead::Event::Finished]");
                state.output_heads.remove(&proxy.id());
            }
            ZwlrOutputHeadEvent::Make { make } => {
                trace!("[OutputHead::Event::Make] {:?}", make);
                head.make = make
            }
            ZwlrOutputHeadEvent::Model { model } => {
                trace!("[OutputHead::Event::Model] {:?}", model);
                head.model = model
            }
            ZwlrOutputHeadEvent::SerialNumber { serial_number } => {
                trace!("[OutputHead::Event::SerialNumber] {:?}", serial_number);
                head.serial_number = serial_number
            }
            _ => {
                warn!("[OutputHead::Event] Unknown event {:?}", event)
            }
        }
    }

    event_created_child!(ShikaneBackend, ZwlrOutputManagerV1, [
        3 => (ZwlrOutputModeV1, Data::default()),
    ]);
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
            _ => warn!("[OutputMode::Event] unknown Event received: {:?}", event),
        }
    }
}

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

impl Dispatch<ZwlrOutputConfigurationV1, Data> for ShikaneBackend {
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

pub(crate) fn connect() -> WaylandSource<ShikaneBackend> {
    let conn = Connection::connect_to_env().unwrap();
    let display = conn.display();
    let event_queue = conn.new_event_queue();
    let qh = event_queue.handle();
    let _registry = display.get_registry(&qh, Data::default()).unwrap();
    WaylandSource::new(event_queue).unwrap()
}
