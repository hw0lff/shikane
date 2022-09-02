mod output_configuration;
mod output_configuration_head;
mod output_head;
mod output_manager;
mod output_mode;
mod wl_registry;

use crate::config::Mode;

use self::output_head::OutputHead;
use self::output_mode::OutputMode;

use std::collections::HashMap;

use smithay_client_toolkit::event_loop::WaylandSource;
use wayland_client::{backend::ObjectId, Connection, Proxy, QueueHandle};
use wayland_client::{DispatchError, EventQueue};
use wayland_protocols_wlr::output_management::v1::client::zwlr_output_configuration_v1::ZwlrOutputConfigurationV1;
use wayland_protocols_wlr::output_management::v1::client::zwlr_output_head_v1::ZwlrOutputHeadV1;
use wayland_protocols_wlr::output_management::v1::client::zwlr_output_manager_v1::ZwlrOutputManagerV1;

#[allow(unused_imports)]
use log::{debug, error, info, trace, warn};
use thiserror::Error;
use wayland_protocols_wlr::output_management::v1::client::zwlr_output_mode_v1::ZwlrOutputModeV1;

#[derive(Debug)]
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
    pub(crate) data: Data,
    pub(crate) connection: Connection,
    pub(crate) qh: QueueHandle<ShikaneBackend>,
}

#[derive(Error, Debug)]
enum ShikaneError {
    #[error("Unable to release resources associated with destroyed mode")]
    ReleaseOutputMode,
}

#[derive(Copy, Clone, Default, Debug)]
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

    pub(crate) fn create_configuration(&mut self) -> ZwlrOutputConfigurationV1 {
        self.wlr_output_manager
            .as_ref()
            .unwrap()
            .create_configuration(self.output_manager_serial, &self.qh, self.data)
            .unwrap()
    }

    pub(crate) fn get_modes_of_head(&self, id: &ObjectId) -> Vec<(ObjectId, &OutputMode)> {
        let head = &self.output_heads[id];

        head.modes
            .iter()
            .filter_map(|id| {
                self.output_modes
                    .contains_key(id)
                    .then_some((id.clone(), &self.output_modes[id]))
            })
            .collect()
    }

    pub(crate) fn match_mode(&self, id: &ObjectId, mode: &Mode) -> Option<(ObjectId, &OutputMode)> {
        self.get_modes_of_head(id)
            .into_iter()
            .find(|(_id, output_mode)| output_mode.matches(mode.width, mode.height, mode.refresh))
    }

    pub(crate) fn match_head(&self, pat: &str) -> Option<(&ObjectId, &OutputHead)> {
        self.output_heads.iter().find(|(_id, h)| h.matches(pat))
    }

    pub(crate) fn mode_from_id(&self, id: ObjectId) -> ZwlrOutputModeV1 {
        ZwlrOutputModeV1::from_id(&self.connection, id).expect("cannot retrieve mode from id")
    }

    pub(crate) fn head_from_id(&self, id: ObjectId) -> ZwlrOutputHeadV1 {
        ZwlrOutputHeadV1::from_id(&self.connection, id).expect("cannot retrieve head from id")
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

    pub(crate) fn connect() -> (Self, WaylandSource<Self>) {
        let connection = Connection::connect_to_env().unwrap();
        let display = connection.display();
        let event_queue = connection.new_event_queue();
        let qh = event_queue.handle();
        let backend = Self {
            connection,
            qh,
            data: Default::default(),
            done: Default::default(),
            output_manager_serial: Default::default(),
            wlr_output_manager: Default::default(),
            output_heads: Default::default(),
            output_modes: Default::default(),
            mode_id_head_id: Default::default(),
        };
        let _registry = display.get_registry(&backend.qh, backend.data).unwrap();

        (backend, WaylandSource::new(event_queue).unwrap())
    }

    pub(crate) fn refresh(&mut self) {
        trace!("[IdleRefresh]");
        self.connection
            .flush()
            .expect("cannot flush wayland connection")
    }
}
