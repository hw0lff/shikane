mod output_configuration;
mod output_configuration_head;
mod output_head;
mod output_manager;
mod output_mode;
mod wl_registry;

use self::output_head::OutputHead;
use self::output_mode::OutputMode;

use std::collections::HashMap;

use smithay_client_toolkit::event_loop::WaylandSource;
use wayland_client::backend::ObjectId;
use wayland_client::Connection;
use wayland_client::{DispatchError, EventQueue};
use wayland_protocols_wlr::output_management::v1::client::zwlr_output_manager_v1::ZwlrOutputManagerV1;

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

pub(crate) fn connect() -> WaylandSource<ShikaneBackend> {
    let conn = Connection::connect_to_env().unwrap();
    let display = conn.display();
    let event_queue = conn.new_event_queue();
    let qh = event_queue.handle();
    let _registry = display.get_registry(&qh, Data::default()).unwrap();
    WaylandSource::new(event_queue).unwrap()
}
