mod om_configuration;
mod om_head;
mod om_manager;
mod om_mode;
mod wl_registry;

use std::collections::VecDeque;

use calloop_wayland_source::WaylandSource;
#[allow(unused_imports)]
use log::{debug, error, info, trace, warn};
use snafu::{prelude::*, Location};
use wayland_client::backend::{ObjectId, WaylandError};
use wayland_client::globals::BindError;
use wayland_client::protocol::wl_output::Transform as WlTransform;
use wayland_client::{Connection, Proxy, QueueHandle};
use wayland_protocols_wlr::output_management::v1::client::zwlr_output_configuration_v1::ZwlrOutputConfigurationV1;
use wayland_protocols_wlr::output_management::v1::client::zwlr_output_head_v1::AdaptiveSyncState as WlAdaptiveSyncState;
use wayland_protocols_wlr::output_management::v1::client::zwlr_output_head_v1::ZwlrOutputHeadV1;
use wayland_protocols_wlr::output_management::v1::client::zwlr_output_manager_v1::ZwlrOutputManagerV1;
use wayland_protocols_wlr::output_management::v1::client::zwlr_output_mode_v1::ZwlrOutputModeV1;

use crate::error;
use crate::matching::Pairing;
use crate::profile::{AdaptiveSyncState, Transform};
use crate::profile::{Mode, Output};
use crate::variant::ValidVariant;
use crate::wl_backend::{
    DeadHeadCtx, DeadModeCtx, ForeignId, HeadCountMismatchCtx, UnknownHeadCtx, UnknownModeCtx,
    WlBackend, WlBackendEvent, WlConfigurationError, WlHead, WlStore,
};

type WlrootsStore = WlStore<ZwlrOutputHeadV1, ZwlrOutputModeV1, ObjectId>;

#[derive(Debug)]
pub struct WlrootsBackend {
    wl_store: WlrootsStore,
    wlr_output_manager: ZwlrOutputManagerV1,
    wlr_output_manager_serial: u32,
    connection: Connection,
    queue_handle: QueueHandle<Self>,
    event_queue: VecDeque<WlBackendEvent>,
}

impl WlBackend for WlrootsBackend {
    fn apply(&mut self, variant: &ValidVariant) -> Result<(), WlConfigurationError> {
        self.configure_variant(variant)?.apply();
        Ok(())
    }

    fn test(&mut self, variant: &ValidVariant) -> Result<(), WlConfigurationError> {
        self.configure_variant(variant)?.test();
        Ok(())
    }

    fn drain_event_queue(&mut self) -> VecDeque<WlBackendEvent> {
        std::mem::take(&mut self.event_queue)
    }

    fn export_heads(&self) -> Option<VecDeque<WlHead>> {
        self.export_wl_heads()
    }

    fn flush(&self) -> Result<(), WaylandError> {
        self.connection.flush()
    }
}

impl WlrootsBackend {
    pub fn connect() -> Result<(Self, WaylandSource<Self>), WlrootsBackendError> {
        let connection = Connection::connect_to_env().context(WaylandConnectionCtx)?;
        let (globals, event_queue) =
            wayland_client::globals::registry_queue_init(&connection).context(RegistryGlobalCtx)?;
        let queue_handle = event_queue.handle();
        let wlr_output_manager: ZwlrOutputManagerV1 = globals
            .bind(&queue_handle, 3..=4, ())
            .map_err(|e| match e {
                BindError::UnsupportedVersion => UnsupportedVersionCtx {}.build(),
                BindError::NotPresent => GlobalNotPresentCtx {}.build(),
            })?;

        let backend = Self {
            wl_store: Default::default(),
            wlr_output_manager,
            wlr_output_manager_serial: Default::default(),
            connection: connection.clone(),
            queue_handle,
            event_queue: Default::default(),
        };

        Ok((backend, WaylandSource::new(connection, event_queue)))
    }

    fn export_wl_heads(&self) -> Option<VecDeque<WlHead>> {
        match self.wl_store.export() {
            Ok(heads) => Some(heads),
            Err(err) => {
                warn!("{}", error::report(&err));
                None
            }
        }
    }

    pub fn queue_event(&mut self, event: WlBackendEvent) {
        self.event_queue.push_back(event)
    }
}

// impl for configuration
impl WlrootsBackend {
    fn create_configuration(&mut self) -> ZwlrOutputConfigurationV1 {
        self.wlr_output_manager.create_configuration(
            self.wlr_output_manager_serial,
            &self.queue_handle,
            (),
        )
    }

    fn configure_variant(
        &mut self,
        variant: &ValidVariant,
    ) -> Result<ZwlrOutputConfigurationV1, WlConfigurationError> {
        let got = variant.pairings.len();
        let have = self.wl_store.heads_count();
        if got != have {
            return HeadCountMismatchCtx { got, have }.fail();
        }
        let wlr_conf = self.create_configuration();
        for pair in variant.pairings.iter() {
            let res = configure_head_with_pairing(
                &wlr_conf,
                pair,
                &self.wl_store,
                &self.queue_handle,
                self.wlr_output_manager.version(),
            );

            if let Err(err) = res {
                wlr_conf.destroy();
                return Err(err);
            }
        }
        Ok(wlr_conf)
    }
}

fn configure_head_with_pairing(
    wlr_conf: &ZwlrOutputConfigurationV1,
    pairing: &Pairing,
    wl_store: &WlrootsStore,
    qh: &QueueHandle<WlrootsBackend>,
    wlr_om_version: u32,
) -> Result<(), WlConfigurationError> {
    let wl_head: &WlHead = pairing.wl_head();
    let output: &Output = pairing.output();
    let wlr_head: &ZwlrOutputHeadV1 = match wl_store.head_store_key(wl_head.id) {
        Ok(store_head) => &store_head.foreign_head,
        Err(err) => {
            warn!("{}", error::report(&err));
            return UnknownHeadCtx {
                head: pairing.wl_head().wl_base_head().clone(),
            }
            .fail();
        }
    };
    // Cannot configure a head that is not alive
    if !wlr_head.is_alive() {
        let head_name = wl_head.name();
        return DeadHeadCtx { head_name }.fail();
    }

    // Disable the head if is disabled in the config
    if !output.enable {
        wlr_conf.disable_head(wlr_head);
        return Ok(());
    }

    // Enable the head and set its properties
    let wlr_conf_head = wlr_conf.enable_head(wlr_head, qh, ());

    // Mode
    if let Some(smode) = output.mode {
        if let Mode::WiHeReCustom(width, height, refresh) = smode {
            trace!("Setting Mode: {smode}");
            wlr_conf_head.set_custom_mode(width, height, refresh);
        } else if let Some(wl_mode) = pairing.wl_mode() {
            let wl_base_mode = wl_mode.wl_base_mode();
            let wlr_mode = match wl_store.mode_store_key(wl_mode.id) {
                Ok(store_mode) => &store_mode.foreign_mode,
                Err(err) => {
                    warn!("{}", error::report(&err));
                    return UnknownModeCtx { mode: wl_base_mode }.fail();
                }
            };
            // Cannot configure a mode that is not alive
            if !wlr_mode.is_alive() {
                return DeadModeCtx { mode: wl_base_mode }.fail();
            }
            trace!("Setting Mode: {smode} | {}", wl_base_mode);
            wlr_conf_head.set_mode(wlr_mode);
        }
    }

    // Position
    if let Some(pos) = output.position {
        trace!("Setting Position: {}", pos);
        wlr_conf_head.set_position(pos.x, pos.y);
    }

    // Scale
    if let Some(scale) = output.scale {
        trace!("Setting Scale: {}", scale);
        wlr_conf_head.set_scale(scale);
    }

    // Transform
    if let Some(transform) = output.transform {
        trace!("Setting Transform: {transform}");
        wlr_conf_head.set_transform(transform.into());
    }

    // Adaptive Sync
    if let Some(adaptive_sync) = output.adaptive_sync {
        if wlr_om_version >= 4 {
            trace!("Setting Adaptive Sync: {adaptive_sync}");
            wlr_conf_head.set_adaptive_sync(adaptive_sync.into());
        } else {
            let msg = format!("Cannot set adaptive_sync to {adaptive_sync}.");
            let msg = format!("{msg} wlr-output-management protocol version >= 4 needed.");
            warn!("{msg} Have version {wlr_om_version}");
        }
    }

    Ok(())
}

impl ForeignId for ZwlrOutputHeadV1 {
    type Id = ObjectId;
    fn foreign_id(&self) -> Self::Id {
        self.id()
    }
}
impl ForeignId for ZwlrOutputModeV1 {
    type Id = ObjectId;
    fn foreign_id(&self) -> Self::Id {
        self.id()
    }
}

impl From<AdaptiveSyncState> for WlAdaptiveSyncState {
    fn from(value: AdaptiveSyncState) -> Self {
        match value {
            AdaptiveSyncState::Disabled => Self::Disabled,
            AdaptiveSyncState::Enabled => Self::Enabled,
        }
    }
}

impl From<Transform> for WlTransform {
    fn from(value: Transform) -> Self {
        match value {
            Transform::Normal => Self::Normal,
            Transform::_90 => Self::_90,
            Transform::_180 => Self::_180,
            Transform::_270 => Self::_270,
            Transform::Flipped => Self::Flipped,
            Transform::Flipped90 => Self::Flipped90,
            Transform::Flipped180 => Self::Flipped180,
            Transform::Flipped270 => Self::Flipped270,
        }
    }
}

#[derive(Debug, Snafu)]
#[snafu(context(suffix(Ctx)))]
#[snafu(visibility(pub(crate)))]
pub enum WlrootsBackendError {
    #[snafu(display("[{location}] wlr-output-management protocol version < 3 is not supported"))]
    UnsupportedVersion { location: Location },
    #[snafu(display("[{location}] wlr_output_manager global not present"))]
    GlobalNotPresent { location: Location },
    #[snafu(display("[{location}] Failed to retrieve Wayland globals from registry"))]
    RegistryGlobal {
        source: wayland_client::globals::GlobalError,
        location: Location,
    },
    #[snafu(display("[{location}] Cannot connect to Wayland server"))]
    WaylandConnection {
        source: wayland_client::ConnectError,
        location: Location,
    },
    #[snafu(display("[{location}] Failed to flush connection to Wayland server"))]
    WaylandConnectionFlush {
        source: wayland_client::backend::WaylandError,
        location: Location,
    },
    #[snafu(display("[{location}] Cannot get wayland object from specified ID"))]
    WaylandInvalidId {
        source: wayland_client::backend::InvalidId,
        location: Location,
    },
    #[snafu(display("[{location}] Unable to release resources associated with destroyed mode"))]
    ReleaseOutputMode { location: Location },
}
