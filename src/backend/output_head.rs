use super::{Data, ShikaneBackend};

use wayland_client::backend::ObjectId;
use wayland_client::event_created_child;
use wayland_client::protocol::wl_output::Transform;
use wayland_client::{Connection, Dispatch, Proxy, QueueHandle};
use wayland_protocols_wlr::output_management::v1::client::zwlr_output_head_v1::Event as ZwlrOutputHeadEvent;
use wayland_protocols_wlr::output_management::v1::client::zwlr_output_head_v1::ZwlrOutputHeadV1;
use wayland_protocols_wlr::output_management::v1::client::zwlr_output_manager_v1::ZwlrOutputManagerV1;
use wayland_protocols_wlr::output_management::v1::client::zwlr_output_mode_v1::ZwlrOutputModeV1;

#[allow(unused_imports)]
use log::{debug, error, info, trace, warn};

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
            }
            ZwlrOutputHeadEvent::Enabled { enabled } => {
                trace!("[Event::Enabled]");
                head.enabled = !matches!(enabled, 0)
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
                head.transform = match transform.into_result() {
                    Ok(transform) => {
                        trace!("[Event::Transform] {:?}", transform);
                        Some(transform)
                    }
                    Err(err) => {
                        warn!(
                        "[Event::Transform] The stored value does not match one defined by the protocol file: {:?}",
                        err
                    );
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
            _ => {
                warn!("[Event] unknown event received {:?}", event)
            }
        }
    }

    event_created_child!(ShikaneBackend, ZwlrOutputManagerV1, [
        3 => (ZwlrOutputModeV1, Data::default()),
    ]);
}

impl OutputHead {
    pub(crate) fn matches(&self, pat: &str) -> bool {
        self.name == pat || self.make == pat || self.model == pat
    }
}
