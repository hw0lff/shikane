#[allow(unused_imports)]
use log::{debug, error, info, trace, warn};
use wayland_client::globals::GlobalListContents;
use wayland_client::{protocol::wl_registry, Connection, Dispatch, QueueHandle};

use super::WlrootsBackend;

impl Dispatch<wl_registry::WlRegistry, GlobalListContents> for WlrootsBackend {
    fn event(
        _: &mut Self,
        _: &wl_registry::WlRegistry,
        _: wl_registry::Event,
        _: &GlobalListContents,
        _: &Connection,
        _: &QueueHandle<Self>,
    ) {
    }
}
