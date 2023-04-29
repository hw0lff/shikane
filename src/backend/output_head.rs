use super::OutputMode;
use super::{Data, ShikaneBackend};

use wayland_client::backend::ObjectId;
use wayland_client::event_created_child;
use wayland_client::protocol::wl_output::Transform;
use wayland_client::{Connection, Dispatch, Proxy, QueueHandle};
use wayland_protocols_wlr::output_management::v1::client::zwlr_output_head_v1::AdaptiveSyncState;
use wayland_protocols_wlr::output_management::v1::client::zwlr_output_head_v1::Event as ZwlrOutputHeadEvent;
use wayland_protocols_wlr::output_management::v1::client::zwlr_output_head_v1::ZwlrOutputHeadV1;
use wayland_protocols_wlr::output_management::v1::client::zwlr_output_head_v1::EVT_MODE_OPCODE;
use wayland_protocols_wlr::output_management::v1::client::zwlr_output_manager_v1::ZwlrOutputManagerV1;
use wayland_protocols_wlr::output_management::v1::client::zwlr_output_mode_v1::ZwlrOutputModeV1;

#[allow(unused_imports)]
use log::{debug, error, info, trace, warn};

#[derive(Clone, Debug, PartialEq)]
pub struct OutputHead {
    pub name: String,
    pub description: String,
    pub physical_width: i32,
    pub physical_height: i32,
    pub modes: Vec<ObjectId>,
    pub enabled: bool,
    pub current_mode: Option<ObjectId>,
    pub position_x: i32,
    pub position_y: i32,
    pub transform: Option<Transform>,
    pub scale: f64,
    pub make: String,
    pub model: String,
    pub serial_number: String,
    pub adaptive_sync: Option<AdaptiveSyncState>,
    pub wlr_head: ZwlrOutputHeadV1,
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
        let w_value_err = "The stored value does not match one defined by the protocol file";

        // Initialize the OutputHead and ensure it is in the HashMap
        let mut head;
        match state.output_heads.get_mut(&proxy.id()) {
            Some(m) => head = m,
            None => {
                state
                    .output_heads
                    .insert(proxy.id(), OutputHead::new(proxy.clone()));
                head = state.output_heads.get_mut(&proxy.id()).unwrap();
            }
        };

        // Update the properties of a head
        match event {
            ZwlrOutputHeadEvent::Name { name } => {
                trace!("[Event::Name] {:?}", name);
                head.name = name
            }
            ZwlrOutputHeadEvent::Description { description } => {
                trace!("[Event::Description] {:?}", description);
                head.description = description
            }
            ZwlrOutputHeadEvent::PhysicalSize { width, height } => {
                trace!(
                    "[Event::PhysicalSize] width: {:?}, height: {:?}",
                    width,
                    height
                );
                (head.physical_width, head.physical_height) = (width, height)
            }
            ZwlrOutputHeadEvent::Mode { mode } => {
                trace!("[Event::Mode] id: {:?}", mode.id());
                state.mode_id_head_id.insert(mode.id(), proxy.id());
                head.modes.push(mode.id());
                state.output_modes.insert(mode.id(), OutputMode::new(mode));
            }
            ZwlrOutputHeadEvent::Enabled { enabled } => {
                head.enabled = !matches!(enabled, 0);
                trace!("[Event::Enabled] {}", head.enabled);
            }
            ZwlrOutputHeadEvent::CurrentMode { mode } => {
                trace!("[Event::CurrentMode] id: {:?}", mode.id());
                head.current_mode = Some(mode.id())
            }
            ZwlrOutputHeadEvent::Position { x, y } => {
                trace!("[Event::Position] x: {:?}, y: {:?}", x, y);
                (head.position_x, head.position_y) = (x, y)
            }
            ZwlrOutputHeadEvent::Transform { transform } => {
                let event_prefix = "[Event::Transform]";
                head.transform = match transform.into_result() {
                    Ok(transform) => {
                        trace!("{event_prefix} {:?}", transform);
                        Some(transform)
                    }
                    Err(err) => {
                        warn!("{event_prefix} {w_value_err}: {:?}", err);
                        None
                    }
                }
            }
            ZwlrOutputHeadEvent::Scale { scale } => {
                trace!("[Event::Scale] {:?}", scale);
                head.scale = scale
            }
            ZwlrOutputHeadEvent::Finished => {
                trace!("[Event::Finished]");
                proxy.release();
                state.output_heads.remove(&proxy.id());
            }
            ZwlrOutputHeadEvent::Make { make } => {
                trace!("[Event::Make] {:?}", make);
                head.make = make
            }
            ZwlrOutputHeadEvent::Model { model } => {
                trace!("[Event::Model] {:?}", model);
                head.model = model
            }
            ZwlrOutputHeadEvent::SerialNumber { serial_number } => {
                trace!("[Event::SerialNumber] {:?}", serial_number);
                head.serial_number = serial_number
            }
            ZwlrOutputHeadEvent::AdaptiveSync { state } => {
                let event_prefix = "[Event::AdaptiveSync]";
                head.adaptive_sync = match state.into_result() {
                    Ok(adaptive_sync) => {
                        trace!("{event_prefix} {:?}", adaptive_sync);
                        Some(adaptive_sync)
                    }
                    Err(err) => {
                        warn!("{event_prefix} {w_value_err}: {:?}", err);
                        None
                    }
                }
            }
            _ => {
                warn!("[Event] unknown event received {:?}", event)
            }
        }
    }

    event_created_child!(ShikaneBackend, ZwlrOutputManagerV1, [
        EVT_MODE_OPCODE => (ZwlrOutputModeV1, Data::default()),
    ]);
}

impl OutputHead {
    pub fn new(wlr_head: ZwlrOutputHeadV1) -> Self {
        Self {
            name: Default::default(),
            description: Default::default(),
            physical_width: Default::default(),
            physical_height: Default::default(),
            modes: Default::default(),
            enabled: Default::default(),
            current_mode: Default::default(),
            position_x: Default::default(),
            position_y: Default::default(),
            transform: Default::default(),
            scale: Default::default(),
            make: Default::default(),
            model: Default::default(),
            serial_number: Default::default(),
            adaptive_sync: Default::default(),
            wlr_head,
        }
    }
}
