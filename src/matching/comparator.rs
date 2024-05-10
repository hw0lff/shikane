use std::collections::VecDeque;

use itertools::Itertools;
#[allow(unused_imports)]
use log::{debug, error, info, trace, warn};

use crate::profile::{Mode, Output};
use crate::wl_backend::{WlHead, WlMode};

use super::{
    IntermediatePairing, IntermediatePairingWithMultipleModes, IntermediatePairingWithoutMode,
    UnrelatedPairing,
};

#[derive(Clone, Copy, Debug)]
pub struct Comparator;

#[derive(Clone, Debug)]
pub struct ComparatorInfo {
    pub(crate) intermediate_pairings: Vec<IntermediatePairing>,
    pub(crate) unpaired_heads: Vec<WlHead>,
    pub(crate) unpaired_outputs: Vec<Output>,
    pub(crate) unrelated_pairings: Vec<UnrelatedPairing>,
}

impl Comparator {
    pub(crate) fn collect_intermediate_pairings<'o, 'h>(
        outputs: impl Iterator<Item = &'o Output> + Clone,
        wl_heads: impl Iterator<Item = &'h WlHead> + Clone,
    ) -> ComparatorInfo {
        let results: Vec<_> = outputs
            .clone()
            .cartesian_product(wl_heads.clone())
            .map(|(output, head)| Self::matcher(output, head))
            .collect();

        let mut intermediate_pairings = vec![];
        let mut unrelated_pairings = vec![];
        for res in results {
            match res {
                ComparatorResult::IntermediatePairing(ipair) => intermediate_pairings.push(ipair),
                ComparatorResult::UnrelatedParing(upair) => unrelated_pairings.push(upair),
            }
        }

        let unpaired_outputs: Vec<_> = outputs
            .filter(|o| {
                !intermediate_pairings
                    .iter()
                    .any(|ipair| ipair.output() == *o)
            })
            .cloned()
            .collect();
        let unpaired_heads: Vec<_> = wl_heads
            .filter(|h| {
                !intermediate_pairings
                    .iter()
                    .any(|ipair| ipair.matched_head() == *h)
            })
            .cloned()
            .collect();

        ComparatorInfo {
            intermediate_pairings,
            unpaired_heads,
            unpaired_outputs,
            unrelated_pairings,
        }
    }

    fn matcher(output: &Output, wl_head: &WlHead) -> ComparatorResult {
        debug!(
            "comparing output \"{}\" with head {:?}",
            output.search_pattern,
            wl_head.name()
        );
        let search_result = output
            .search_pattern
            .clone()
            .query()
            .description(wl_head.description())
            .name(wl_head.name())
            .model(wl_head.model())
            .serial(wl_head.serial_number())
            .vendor(wl_head.make())
            .run();
        debug!("search_result.is_ok={}", search_result.is_ok());

        // If we don't have to find a mode (no mode specified or custom mode),
        // we cannot run into the "unsupported mode" case. We can stop here.
        if output.mode.is_none() || output.mode.is_some_and(|m| m.is_custom()) {
            if !search_result.is_ok() {
                let mut upair = UnrelatedPairing::new(output.clone(), wl_head.clone());
                upair.failed_search(search_result);
                return upair.into();
            }
            let pair =
                IntermediatePairingWithoutMode::new(search_result, output.clone(), wl_head.clone());
            return pair.into();
        }

        // Unwrap is ok here because of early return
        let smode = output.mode.unwrap();
        let matched_modes = collect_modes(wl_head.modes().clone(), &smode).unwrap();

        if matched_modes.is_empty() || !search_result.is_ok() {
            let mut upair = UnrelatedPairing::new(output.clone(), wl_head.clone());
            if !search_result.is_ok() {
                upair.failed_search(search_result);
            }
            if matched_modes.is_empty() {
                upair.unsupported_mode(smode);
            }
            return upair.into();
        }

        IntermediatePairingWithMultipleModes::new(
            search_result,
            output.clone(),
            wl_head.clone(),
            matched_modes,
        )
        .into()
    }
}

enum ComparatorResult {
    IntermediatePairing(IntermediatePairing),
    UnrelatedParing(UnrelatedPairing),
}

impl From<UnrelatedPairing> for ComparatorResult {
    fn from(value: UnrelatedPairing) -> Self {
        Self::UnrelatedParing(value)
    }
}
impl From<IntermediatePairingWithoutMode> for ComparatorResult {
    fn from(value: IntermediatePairingWithoutMode) -> Self {
        Self::IntermediatePairing(value.into())
    }
}
impl From<IntermediatePairingWithMultipleModes> for ComparatorResult {
    fn from(value: IntermediatePairingWithMultipleModes) -> Self {
        Self::IntermediatePairing(value.into())
    }
}

/// Collect modes that match the given mode.
fn collect_modes(mut modes: VecDeque<WlMode>, smode: &Mode) -> Option<Vec<WlMode>> {
    sort_modes(modes.make_contiguous());
    modes.make_contiguous().reverse();

    let best = modes.front().cloned();
    let preferred = modes.iter().find(|m| m.preferred()).cloned();
    let preferred = preferred.or(best.clone()).into_iter().collect();
    let best = best.into_iter().collect();

    match smode {
        Mode::Best => Some(best),
        Mode::Preferred => Some(preferred),
        Mode::WiHe(w, h) => Some(
            modes
                .into_iter()
                .filter(|m| m.width() == *w && m.height() == *h)
                .collect(),
        ),
        Mode::WiHeRe(w, h, r) => Some(
            modes
                .into_iter()
                .filter(|m| m.width() == *w && m.height() == *h && compare_mode_refresh(*r, m).0)
                .collect(),
        ),
        Mode::WiHeReCustom(_, _, _) => None,
    }
}

fn compare_mode_refresh(refresh: i32, mode: &WlMode) -> (bool, i32) {
    let diff: i32 = refresh.abs_diff(mode.refresh()) as i32; // difference in mHz
    trace!(
        "refresh: {refresh}mHz, monitor.refresh {}mHz, diff: {diff}mHz",
        mode.refresh()
    );
    (diff <= super::MAX_RR_DEVIATION, diff)
}

fn sort_modes(modes: &mut [WlMode]) {
    modes.sort_by_key(|m| {
        let pixels = m.width() * m.height();
        let w = m.width();
        let h = m.height();
        let r = m.refresh();

        (pixels, w, h, r)
    });
}
