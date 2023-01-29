mod output_configuration;
mod output_configuration_head;
pub mod output_head;
mod output_manager;
pub mod output_mode;
mod wl_registry;

use crate::error::ShikaneError;
use crate::profile::Mode;
use crate::state::StateInput;

use self::output_head::OutputHead;
use self::output_mode::OutputMode;

use std::collections::HashMap;

use calloop::channel::Sender;
use wayland_client::WaylandSource;
use wayland_client::{backend::ObjectId, Connection, Proxy, QueueHandle};
use wayland_client::{DispatchError, EventQueue};
use wayland_protocols_wlr::output_management::v1::client::zwlr_output_configuration_v1::ZwlrOutputConfigurationV1;
use wayland_protocols_wlr::output_management::v1::client::zwlr_output_head_v1::ZwlrOutputHeadV1;
use wayland_protocols_wlr::output_management::v1::client::zwlr_output_manager_v1::ZwlrOutputManagerV1;

#[allow(unused_imports)]
use log::{debug, error, info, trace, warn};
use wayland_protocols_wlr::output_management::v1::client::zwlr_output_mode_v1::ZwlrOutputModeV1;

#[derive(Debug)]
pub struct ShikaneBackend {
    pub output_manager_serial: u32,
    pub wlr_output_manager: Option<ZwlrOutputManagerV1>,
    pub wlr_output_configuration: Option<ZwlrOutputConfigurationV1>,
    /// A Mapping from ZwlrOutputHeadV1-Ids to OutputHeads
    pub output_heads: HashMap<ObjectId, OutputHead>,
    /// A Mapping from ZwlrOutputModeV1-Ids to OutputModes
    pub output_modes: HashMap<ObjectId, OutputMode>,
    /// A Mapping from ZwlrOutputModeV1-Ids to ZwlrOutputHeadV1-Ids
    ///
    /// The ZwlrOutputModeV1 from the key-field belongs to the ZwlrOutputHeadV1 in the value-field
    pub mode_id_head_id: HashMap<ObjectId, ObjectId>,
    pub data: Data,
    pub connection: Connection,
    pub qh: QueueHandle<ShikaneBackend>,
    sender: Sender<StateInput>,
}

#[derive(Copy, Clone, Default, Debug)]
pub struct Data;

impl ShikaneBackend {
    pub fn callback(
        &mut self,
        event_queue: &mut EventQueue<ShikaneBackend>,
    ) -> Result<usize, DispatchError> {
        let dispatch_result = event_queue.dispatch_pending(self);
        trace!("[Dispatch::Result] {:?}", dispatch_result);
        dispatch_result
    }

    pub fn send(&mut self, event: StateInput) {
        self.sender
            .send(event)
            .expect("cannot send input to state machine");
    }

    pub fn create_configuration(&mut self) -> ZwlrOutputConfigurationV1 {
        self.destroy_configuration();
        let configuration = self
            .wlr_output_manager
            .as_ref()
            .unwrap()
            .create_configuration(self.output_manager_serial, &self.qh, self.data);
        self.wlr_output_configuration = Some(configuration.clone());
        configuration
    }

    pub fn destroy_configuration(&mut self) {
        if let Some(config) = &self.wlr_output_configuration {
            config.destroy();
            self.wlr_output_configuration = None;
        }
    }

    pub fn heads(&self) -> Vec<&OutputHead> {
        let o_heads: Vec<&OutputHead> = self.output_heads.values().collect();
        o_heads
    }

    pub fn match_mode(&self, o_head: &OutputHead, mode: &Mode) -> Option<&OutputMode> {
        let mut best = None;
        let mut refresh_delta = i32::MAX; // in mHz
        for wlr_mode_id in o_head.modes.iter() {
            let o_mode = self.output_modes.get(wlr_mode_id)?;
            if mode.width != o_mode.width || mode.height != o_mode.height {
                continue;
            }

            if mode.matches(o_mode, &mut refresh_delta) {
                best = Some(o_mode);
            }
        }
        best
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

    pub fn connect(
        sender: Sender<StateInput>,
    ) -> Result<(Self, WaylandSource<Self>), ShikaneError> {
        let connection = Connection::connect_to_env()?;
        let display = connection.display();
        let event_queue = connection.new_event_queue();
        let qh = event_queue.handle();
        let backend = Self {
            connection,
            qh,
            data: Default::default(),
            output_manager_serial: Default::default(),
            wlr_output_manager: Default::default(),
            wlr_output_configuration: Default::default(),
            output_heads: Default::default(),
            output_modes: Default::default(),
            mode_id_head_id: Default::default(),
            sender,
        };
        let _registry = display.get_registry(&backend.qh, backend.data);

        Ok((backend, WaylandSource::new(event_queue)?))
    }

    pub fn flush(&mut self) -> Result<(), ShikaneError> {
        Ok(self.connection.flush()?)
    }

    pub fn clean_up(&mut self) {
        self.destroy_configuration();
        for (id, _) in self.output_modes.drain() {
            match mode_from_id(&self.connection, id) {
                Ok(it) => it.release(),
                Err(err) => warn!("{}", err),
            }
        }
        for (id, _) in self.output_heads.drain() {
            match head_from_id(&self.connection, id) {
                Ok(it) => it.release(),
                Err(err) => warn!("{}", err),
            }
        }
        if let Some(om) = &self.wlr_output_manager {
            om.stop();
        }
    }
}

fn head_from_id(conn: &Connection, id: ObjectId) -> Result<ZwlrOutputHeadV1, ShikaneError> {
    Ok(ZwlrOutputHeadV1::from_id(conn, id)?)
}

fn mode_from_id(conn: &Connection, id: ObjectId) -> Result<ZwlrOutputModeV1, ShikaneError> {
    Ok(ZwlrOutputModeV1::from_id(conn, id)?)
}
