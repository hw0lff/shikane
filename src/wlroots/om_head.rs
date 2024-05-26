#[allow(unused_imports)]
use log::{debug, error, info, trace, warn};
use wayland_client::protocol::wl_output::Transform as WlTransform;
use wayland_client::{event_created_child, WEnum};
use wayland_client::{Connection, Dispatch, Proxy, QueueHandle};
use wayland_protocols_wlr::output_management::v1::client::zwlr_output_head_v1::AdaptiveSyncState as ZwlrAdaptiveSyncState;
use wayland_protocols_wlr::output_management::v1::client::zwlr_output_head_v1::Event as ZwlrOutputHeadEvent;
use wayland_protocols_wlr::output_management::v1::client::zwlr_output_head_v1::ZwlrOutputHeadV1;
use wayland_protocols_wlr::output_management::v1::client::zwlr_output_head_v1::EVT_MODE_OPCODE;
use wayland_protocols_wlr::output_management::v1::client::zwlr_output_manager_v1::ZwlrOutputManagerV1;
use wayland_protocols_wlr::output_management::v1::client::zwlr_output_mode_v1::ZwlrOutputModeV1;

use crate::error;
use crate::profile::{AdaptiveSyncState, Transform};

use super::WlrootsBackend;

impl Dispatch<ZwlrOutputHeadV1, ()> for WlrootsBackend {
    fn event(
        backend: &mut Self,
        wlr_head: &ZwlrOutputHeadV1,
        event: <ZwlrOutputHeadV1 as Proxy>::Event,
        _: &(),
        _: &Connection,
        _: &QueueHandle<Self>,
    ) {
        let head = match backend.wl_store.head_mut(wlr_head.id()) {
            Ok(head) => head,
            Err(err) => {
                warn!("{}", error::report(&err));
                return;
            }
        };

        // Update the properties of a head
        match event {
            ZwlrOutputHeadEvent::Name { name } => {
                trace!("[Event::Name] {:?}", name);
                head.base.name = name
            }
            ZwlrOutputHeadEvent::Description { description } => {
                trace!("[Event::Description] {:?}", description);
                head.base.description = description
            }
            ZwlrOutputHeadEvent::PhysicalSize { width, height } => {
                trace!(
                    "[Event::PhysicalSize] width: {:?}, height: {:?}",
                    width,
                    height
                );
                (head.base.size.width, head.base.size.height) = (width, height)
            }
            ZwlrOutputHeadEvent::Mode { mode } => {
                trace!("[Event::Mode] id: {:?}", mode.id());
                backend.wl_store.insert_mode(wlr_head.id(), mode);
            }
            ZwlrOutputHeadEvent::Enabled { enabled } => {
                head.base.enabled = !matches!(enabled, 0);
                trace!("[Event::Enabled] {}", head.base.enabled);
            }
            ZwlrOutputHeadEvent::CurrentMode { mode } => {
                trace!("[Event::CurrentMode] id: {:?}", mode.id());
                head.current_mode = Some(mode.id())
            }
            ZwlrOutputHeadEvent::Position { x, y } => {
                trace!("[Event::Position] x: {:?}, y: {:?}", x, y);
                (head.base.position.x, head.base.position.y) = (x, y)
            }
            ZwlrOutputHeadEvent::Transform { transform } => {
                let event_prefix = "[Event::Transform]";
                let transform = wenum_extract(event_prefix, transform)
                    .and_then(|wlr_transform| transform_try_into(event_prefix, wlr_transform));
                head.base.transform = transform
            }
            ZwlrOutputHeadEvent::Scale { scale } => {
                trace!("[Event::Scale] {:?}", scale);
                head.base.scale = scale
            }
            ZwlrOutputHeadEvent::Finished => {
                trace!("[Event::Finished]");
                wlr_head.release();
                backend.wl_store.remove_head(&wlr_head.id());
            }
            ZwlrOutputHeadEvent::Make { make } => {
                trace!("[Event::Make] {:?}", make);
                head.base.make = make
            }
            ZwlrOutputHeadEvent::Model { model } => {
                trace!("[Event::Model] {:?}", model);
                head.base.model = model
            }
            ZwlrOutputHeadEvent::SerialNumber { serial_number } => {
                trace!("[Event::SerialNumber] {:?}", serial_number);
                head.base.serial_number = serial_number
            }
            ZwlrOutputHeadEvent::AdaptiveSync { state } => {
                let event_prefix = "[Event::AdaptiveSync]";
                let ass = wenum_extract(event_prefix, state)
                    .and_then(|wlr_ass| ass_try_into(event_prefix, wlr_ass));
                head.base.adaptive_sync = ass;
            }
            unknown => {
                warn!("[Event] unknown event received {unknown:?}")
            }
        }
    }

    event_created_child!(WlrootsBackend, ZwlrOutputManagerV1, [
        EVT_MODE_OPCODE => (ZwlrOutputModeV1, ()),
    ]);
}

fn wenum_extract<T: std::fmt::Debug>(event_prefix: &str, wenum: WEnum<T>) -> Option<T> {
    let w_value_err = "The stored value does not match one defined by the protocol file";
    match wenum.into_result() {
        Ok(inner) => {
            trace!("{event_prefix} {:?}", inner);
            Some(inner)
        }
        Err(err) => {
            warn!("{event_prefix} {w_value_err}: {:?}", err);
            None
        }
    }
}

fn ass_try_into(event_prefix: &str, wlr_ass: ZwlrAdaptiveSyncState) -> Option<AdaptiveSyncState> {
    match wlr_ass {
        ZwlrAdaptiveSyncState::Disabled => Some(AdaptiveSyncState::Disabled),
        ZwlrAdaptiveSyncState::Enabled => Some(AdaptiveSyncState::Enabled),
        unknown => {
            warn!("{event_prefix} unknown adaptive sync state: {unknown:?}");
            None
        }
    }
}

fn transform_try_into(event_prefix: &str, wl_transform: WlTransform) -> Option<Transform> {
    match wl_transform {
        WlTransform::Normal => Some(Transform::Normal),
        WlTransform::_90 => Some(Transform::_90),
        WlTransform::_180 => Some(Transform::_180),
        WlTransform::_270 => Some(Transform::_270),
        WlTransform::Flipped => Some(Transform::Flipped),
        WlTransform::Flipped90 => Some(Transform::Flipped90),
        WlTransform::Flipped180 => Some(Transform::Flipped180),
        WlTransform::Flipped270 => Some(Transform::Flipped270),
        unknown => {
            warn!("{event_prefix} unknown transform: {unknown:?}");
            None
        }
    }
}
