mod wl_store;

use std::collections::VecDeque;
use std::fmt::Display;
use std::hash::Hash;

use serde::{Deserialize, Serialize};
use snafu::{prelude::*, Location};
use wayland_client::backend::WaylandError;

use crate::profile::{AdaptiveSyncState, PhysicalSize, Position, Transform};
use crate::variant::ValidVariant;

pub use self::wl_store::{ForeignId, WlStore};

#[derive(Clone, Copy, Debug, Hash, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub struct WlGenericId(usize);

pub trait WlBackend {
    fn apply(&mut self, variant: &ValidVariant) -> Result<(), WlConfigurationError>;
    fn test(&mut self, variant: &ValidVariant) -> Result<(), WlConfigurationError>;

    fn drain_event_queue(&mut self) -> VecDeque<WlBackendEvent>;
    fn export_heads(&self) -> Option<VecDeque<WlHead>>;
    fn flush(&self) -> Result<(), WaylandError>;
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct WlHead {
    base: WlBaseHead,
    current_mode: Option<WlMode>,
    modes: VecDeque<WlMode>,
    pub(crate) id: WlGenericId,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct WlMode {
    base: WlBaseMode,
    pub(crate) id: WlGenericId,
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct WlBaseHead {
    pub name: String,
    pub description: String,
    pub size: PhysicalSize,
    pub enabled: bool,
    pub position: Position,
    pub transform: Option<Transform>,
    pub scale: f64,
    pub make: String,
    pub model: String,
    pub serial_number: String,
    pub adaptive_sync: Option<AdaptiveSyncState>,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct WlBaseMode {
    pub width: i32,
    pub height: i32,
    pub refresh: i32,
    pub preferred: bool,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum WlBackendEvent {
    AtomicChangeDone,
    NeededResourceFinished,
    Succeeded,
    Failed,
    Cancelled,
}

#[derive(Debug, Snafu)]
#[snafu(context(suffix(Ctx)))]
#[snafu(visibility(pub(crate)))]
pub enum WlConfigurationError {
    #[snafu(display(
        "[{location}] Incompatible number of heads supplied: got {got}, have {have}"
    ))]
    HeadCountMismatch {
        location: Location,
        got: usize,
        have: usize,
    },
    #[snafu(display("[{location}] Cannot configure a dead head, head name: {head_name}"))]
    DeadHead {
        location: Location,
        head_name: String,
    },
    #[snafu(display("[{location}] Cannot configure a dead mode, mode: {mode}"))]
    DeadMode {
        location: Location,
        mode: WlBaseMode,
    },
    #[snafu(display("[{location}] Cannot configure an unknown head, head: {head:?}"))]
    UnknownHead {
        location: Location,
        head: Box<WlBaseHead>,
    },
    #[snafu(display("[{location}] Cannot configure an unknown mode, mode: {mode}"))]
    UnknownMode {
        location: Location,
        mode: WlBaseMode,
    },
}

impl WlHead {
    pub fn name(&self) -> &str {
        &self.base.name
    }
    pub fn description(&self) -> &str {
        &self.base.description
    }
    pub fn size(&self) -> PhysicalSize {
        self.base.size
    }
    pub fn modes(&self) -> &VecDeque<WlMode> {
        &self.modes
    }
    pub fn enabled(&self) -> bool {
        self.base.enabled
    }
    pub fn current_mode(&self) -> &Option<WlMode> {
        &self.current_mode
    }
    pub fn position(&self) -> Position {
        self.base.position
    }
    pub fn transform(&self) -> Option<Transform> {
        self.base.transform
    }
    pub fn scale(&self) -> f64 {
        self.base.scale
    }
    pub fn make(&self) -> &str {
        &self.base.make
    }
    pub fn model(&self) -> &str {
        &self.base.model
    }
    pub fn serial_number(&self) -> &str {
        &self.base.serial_number
    }
    pub fn adaptive_sync(&self) -> Option<AdaptiveSyncState> {
        self.base.adaptive_sync
    }
    pub fn wl_base_head(&self) -> &WlBaseHead {
        &self.base
    }
}

impl WlMode {
    pub fn width(&self) -> i32 {
        self.base.width
    }
    pub fn height(&self) -> i32 {
        self.base.height
    }
    pub fn refresh(&self) -> i32 {
        self.base.refresh
    }
    pub fn preferred(&self) -> bool {
        self.base.preferred
    }
    pub fn wl_base_mode(&self) -> WlBaseMode {
        self.base
    }
}

impl Display for WlBaseMode {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}x{}@{}mHz", self.width, self.height, self.refresh)
    }
}

impl Display for WlBackendEvent {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let text = match self {
            WlBackendEvent::AtomicChangeDone => "AtomicChangeDone",
            WlBackendEvent::NeededResourceFinished => "NeededResourceFinished",
            WlBackendEvent::Succeeded => "Succeeded",
            WlBackendEvent::Failed => "Failed",
            WlBackendEvent::Cancelled => "Cancelled",
        };
        write!(f, "{text}")
    }
}

#[derive(Debug)]
pub struct LessEqWlHead<'a>(pub &'a WlHead);

impl<'a> PartialEq for LessEqWlHead<'a> {
    fn eq(&self, other: &Self) -> bool {
        let (a, b) = (self.0, other.0);
        a.id == b.id
            && a.serial_number() == b.serial_number()
            && a.model() == b.model()
            && a.make() == b.make()
            && a.description() == b.description()
            && a.name() == b.name()
    }
}

impl<'a> Eq for LessEqWlHead<'a> {}

impl<'a> Hash for LessEqWlHead<'a> {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.0.id.hash(state);
        self.0.serial_number().hash(state);
        self.0.model().hash(state);
        self.0.make().hash(state);
        self.0.description().hash(state);
        self.0.name().hash(state);
    }
}
