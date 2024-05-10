use serde::{Deserialize, Serialize};

use crate::profile::{Mode, Output};
use crate::search::SearchResult;
use crate::wl_backend::{WlHead, WlMode};

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub enum Pairing {
    WithMode(PairingWithMode),
    WithoutMode(PairingWithoutMode),
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct PairingWithMode {
    pub(crate) search_result: SearchResult,
    pub(crate) output: Output,
    pub(crate) wl_head: WlHead,
    pub(crate) wl_mode: WlMode,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct PairingWithoutMode {
    pub(crate) search_result: SearchResult,
    pub(crate) output: Output,
    pub(crate) wl_head: WlHead,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub enum IntermediatePairing {
    WithMultipleModes(IntermediatePairingWithMultipleModes),
    WithoutMode(IntermediatePairingWithoutMode),
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct IntermediatePairingWithMultipleModes {
    pub(crate) search_result: SearchResult,
    pub(crate) output: Output,
    pub(crate) matched_head: WlHead,
    pub(crate) matched_modes: Vec<WlMode>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct IntermediatePairingWithoutMode {
    pub(crate) search_result: SearchResult,
    pub(crate) output: Output,
    pub(crate) matched_head: WlHead,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct UnrelatedPairing {
    pub(crate) output: Output,
    pub(crate) wl_head: WlHead,
    pub(crate) failed_search: Option<SearchResult>,
    pub(crate) unsupported_mode: Option<Mode>,
}

impl Pairing {
    pub fn output(&self) -> &Output {
        match self {
            Self::WithMode(p) => &p.output,
            Self::WithoutMode(p) => &p.output,
        }
    }
    pub fn custom_mode(&self) -> Option<Mode> {
        match self {
            Self::WithMode(_) => None,
            Self::WithoutMode(p) => match p.output.mode.is_some_and(|m| m.is_custom()) {
                true => p.output.mode,
                false => None,
            },
        }
    }
    pub fn wl_head(&self) -> &WlHead {
        match self {
            Self::WithMode(p) => &p.wl_head,
            Self::WithoutMode(p) => &p.wl_head,
        }
    }
    pub fn wl_mode(&self) -> Option<&WlMode> {
        match self {
            Self::WithMode(p) => Some(&p.wl_mode),
            Self::WithoutMode(_) => None,
        }
    }
    /// Return how much the refresh rate of the matched [`WlMode`] deviates from the specified [`Mode`].
    /// Lower is better.
    pub fn mode_deviation(&self) -> u32 {
        match self {
            Pairing::WithMode(p) => {
                if let Some(o_refresh) = p.output.mode.and_then(|m| m.refresh()) {
                    (p.wl_mode.refresh() - o_refresh.wrapping_abs()).unsigned_abs()
                } else {
                    0
                }
            }
            Pairing::WithoutMode(_) => 0,
        }
    }
    /// Return how specific the [`Output`] matches to the [`WlHead`].
    /// Higher is better.
    pub fn specificity(&self) -> u64 {
        match self {
            Pairing::WithMode(p) => p.search_result.specificity(),
            Pairing::WithoutMode(p) => p.search_result.specificity(),
        }
    }
}

impl IntermediatePairing {
    pub fn output(&self) -> &Output {
        match self {
            IntermediatePairing::WithMultipleModes(ip) => &ip.output,
            IntermediatePairing::WithoutMode(ip) => &ip.output,
        }
    }
    pub fn matched_head(&self) -> &WlHead {
        match self {
            IntermediatePairing::WithMultipleModes(ip) => &ip.matched_head,
            IntermediatePairing::WithoutMode(ip) => &ip.matched_head,
        }
    }
    pub(super) fn expand(self) -> Vec<Pairing> {
        match self {
            IntermediatePairing::WithMultipleModes(ipair) => ipair.expand(),
            IntermediatePairing::WithoutMode(ipair) => vec![PairingWithoutMode {
                search_result: ipair.search_result,
                output: ipair.output,
                wl_head: ipair.matched_head,
            }
            .into()],
        }
    }
}

impl IntermediatePairingWithMultipleModes {
    pub(super) fn new(
        search_result: SearchResult,
        output: Output,
        matched_head: WlHead,
        matched_modes: Vec<WlMode>,
    ) -> Self {
        Self {
            search_result,
            output,
            matched_head,
            matched_modes,
        }
    }
    pub(super) fn expand(self) -> Vec<Pairing> {
        self.matched_modes
            .into_iter()
            .map(|m| {
                PairingWithMode {
                    search_result: self.search_result.clone(),
                    output: self.output.clone(),
                    wl_head: self.matched_head.clone(),
                    wl_mode: m,
                }
                .into()
            })
            .collect()
    }
}

impl IntermediatePairingWithoutMode {
    pub(super) fn new(search_result: SearchResult, output: Output, matched_head: WlHead) -> Self {
        Self {
            search_result,
            output,
            matched_head,
        }
    }
}

impl UnrelatedPairing {
    pub(super) fn new(output: Output, wl_head: WlHead) -> Self {
        Self {
            output,
            wl_head,
            failed_search: Default::default(),
            unsupported_mode: Default::default(),
        }
    }
    pub(super) fn failed_search(&mut self, search_result: SearchResult) -> &mut Self {
        self.failed_search = Some(search_result);
        self
    }
    pub(super) fn unsupported_mode(&mut self, mode: Mode) -> &mut Self {
        self.unsupported_mode = Some(mode);
        self
    }
}

impl From<PairingWithMode> for Pairing {
    fn from(value: PairingWithMode) -> Self {
        Self::WithMode(value)
    }
}

impl From<PairingWithoutMode> for Pairing {
    fn from(value: PairingWithoutMode) -> Self {
        Self::WithoutMode(value)
    }
}

impl From<IntermediatePairingWithMultipleModes> for IntermediatePairing {
    fn from(value: IntermediatePairingWithMultipleModes) -> Self {
        Self::WithMultipleModes(value)
    }
}

impl From<IntermediatePairingWithoutMode> for IntermediatePairing {
    fn from(value: IntermediatePairingWithoutMode) -> Self {
        Self::WithoutMode(value)
    }
}
