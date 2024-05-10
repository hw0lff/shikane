use std::collections::VecDeque;

use itertools::Itertools;
#[allow(unused_imports)]
use log::{debug, error, info, trace, warn};
use snafu::{prelude::*, Location};

use crate::pipeline::PipeStage;
use crate::profile::{Output, Profile};
use crate::variant::ValidVariant;
use crate::wl_backend::WlHead;

use super::{
    Comparator, ComparatorInfo, HopcroftKarpMap, IntermediatePairing, MatchReport, UnrelatedPairing,
};

/// Create initial pairings
pub struct Stage1;
/// Send combinations of pairings through Hopcroft-Karp
pub struct Stage2;
/// Expand pairings to variants
pub struct Stage3;

/// Input for the matching pipeline
#[derive(Clone, Debug)]
pub struct MatcherInput {
    pub(super) wl_heads: VecDeque<WlHead>,
    size: usize,

    profile: Profile,
    outputs: Vec<Output>,
}
/// Data transfer between stage 1 and stage 2
#[derive(Clone, Debug)]
pub struct TransferOneTwo {
    initial: MatcherInput,

    intermediate_pairings: Vec<IntermediatePairing>,
    unpaired_heads: Vec<WlHead>,
    unpaired_outputs: Vec<Output>,
    unrelated_pairings: Vec<UnrelatedPairing>,
}
/// Data transfer between stage 2 and stage 3
#[derive(Clone, Debug)]
pub struct TransferTwoThree {
    initial: MatcherInput,

    unpaired_heads: Vec<WlHead>,
    unpaired_outputs: Vec<Output>,
    unrelated_pairings: Vec<UnrelatedPairing>,

    valid_subsets: Vec<Vec<IntermediatePairing>>,
    invalid_subsets: Vec<Vec<IntermediatePairing>>,
}
/// Output of the matching pipeline
#[derive(Clone, Debug)]
pub struct MatcherOutput {
    initial: MatcherInput,
    // from stage 1
    unpaired_heads: Vec<WlHead>,
    unpaired_outputs: Vec<Output>,
    unrelated_pairings: Vec<UnrelatedPairing>,
    // from stage 2
    // contains lists of pairings where [pairing].len() != wl_heads.len()
    invalid_subsets: Vec<Vec<IntermediatePairing>>,
    // from stage 3
    valid_variants: VecDeque<ValidVariant>,
}

impl PipeStage for Stage1 {
    type Input = MatcherInput;
    type Output = TransferOneTwo;
    type Error = MatchPipelineError;

    fn process(input: Self::Input) -> Result<Self::Output, Self::Error> {
        info!("stage 1");
        debug!("profile: {}", input.profile.name);

        if input.outputs.len() != input.wl_heads.len() {
            return DifferentInputLengthCtx { input }.fail();
        }
        let info =
            Comparator::collect_intermediate_pairings(input.outputs.iter(), input.wl_heads.iter());

        let ipair_len = info.intermediate_pairings.len();
        let valid_ipair_count = input.size <= ipair_len && ipair_len <= (input.size * input.size);
        debug!("len(unrelated pairs)={:?}", info.unrelated_pairings.len());
        debug!("len(unpaired outputs)={:?}", info.unpaired_outputs.len());
        debug!("len(unpaired heads)={:?}", info.unpaired_heads.len());
        debug!("len(intermediate pairs)={:?}", ipair_len);
        debug!("len(ipairs) ∈ [size;(size*size)] -> {}", valid_ipair_count);
        if !valid_ipair_count {
            let transfer = input.enrich(info);
            return NotEnoughPairingsCtx { transfer }.fail();
        }
        Ok(input.enrich(info))
    }
}

impl PipeStage for Stage2 {
    type Input = TransferOneTwo;
    type Output = TransferTwoThree;
    type Error = MatchPipelineError;

    fn process(mut input: Self::Input) -> Result<Self::Output, Self::Error> {
        info!("stage 2");
        let (valid_subsets, invalid_subsets): (Vec<Vec<_>>, Vec<Vec<_>>) =
            std::mem::take(&mut input.intermediate_pairings)
                .into_iter()
                // Create k-element subsets of intermediate pairings, with k == wl_heads.len()
                .combinations(input.initial.size)
                .map(|ipairs| ipairs.into_iter())
                .map(HopcroftKarpMap::hkmap)
                .map(|i| i.collect::<Vec<_>>())
                .partition(|ipairs| ipairs.len() == input.initial.size);

        if valid_subsets.is_empty() {
            let transfer = input.enrich(valid_subsets, invalid_subsets);
            return LowCardinalityCtx { transfer }.fail();
        }

        Ok(input.enrich(valid_subsets, invalid_subsets))
    }
}

impl PipeStage for Stage3 {
    type Input = TransferTwoThree;
    type Output = MatcherOutput;
    type Error = MatchPipelineError;

    fn process(mut input: Self::Input) -> Result<Self::Output, Self::Error> {
        info!("stage 3");
        let valid_variants: VecDeque<ValidVariant> = std::mem::take(&mut input.valid_subsets)
            .into_iter()
            .map(|ipairs| {
                ipairs
                    .into_iter()
                    .map(|ipair| ipair.expand())
                    .multi_cartesian_product()
            })
            .enumerate()
            .flat_map(|(idx, pairs)| {
                pairs.into_iter().map(move |p| (idx, p)).enumerate().map(
                    |(jdx, (idx, pairings))| ValidVariant {
                        profile: input.initial.profile.clone(),
                        pairings,
                        state: Default::default(),
                        index: (idx * input.initial.size + jdx),
                    },
                )
            })
            .collect();

        Ok(input.enrich(valid_variants))
    }
}

impl MatcherInput {
    pub fn new(wl_heads: VecDeque<WlHead>, profile: Profile) -> Self {
        let size = wl_heads.len();
        Self {
            wl_heads,
            size,
            outputs: profile.outputs.clone(),
            profile,
        }
    }
    fn enrich(self, info: ComparatorInfo) -> TransferOneTwo {
        TransferOneTwo {
            initial: self,
            intermediate_pairings: info.intermediate_pairings,
            unpaired_heads: info.unpaired_heads,
            unpaired_outputs: info.unpaired_outputs,
            unrelated_pairings: info.unrelated_pairings,
        }
    }
}
impl TransferOneTwo {
    fn enrich(
        self,
        valid_subsets: Vec<Vec<IntermediatePairing>>,
        invalid_subsets: Vec<Vec<IntermediatePairing>>,
    ) -> TransferTwoThree {
        TransferTwoThree {
            initial: self.initial,
            unpaired_heads: self.unpaired_heads,
            unpaired_outputs: self.unpaired_outputs,
            unrelated_pairings: self.unrelated_pairings,
            valid_subsets,
            invalid_subsets,
        }
    }
}
impl TransferTwoThree {
    fn enrich(self, valid_variants: VecDeque<ValidVariant>) -> MatcherOutput {
        MatcherOutput {
            initial: self.initial,
            unpaired_heads: self.unpaired_heads,
            unpaired_outputs: self.unpaired_outputs,
            unrelated_pairings: self.unrelated_pairings,
            invalid_subsets: self.invalid_subsets,
            valid_variants,
        }
    }
}
impl MatcherOutput {
    pub(super) fn into_report(self) -> MatchReport {
        MatchReport {
            profile: self.initial.profile,
            wl_heads: self.initial.wl_heads,
            unpaired_heads: self.unpaired_heads,
            unpaired_outputs: self.unpaired_outputs,
            unrelated_pairings: self.unrelated_pairings,
            invalid_subsets: self.invalid_subsets,
            valid_variants: self.valid_variants,
        }
    }
}

/// Errors that may occur in the matching pipeline.
#[derive(Debug, Snafu)]
#[snafu(context(suffix(Ctx)))]
pub enum MatchPipelineError {
    /// occurs in step 1
    #[snafu(display("[{location}] Mismatched count of outputs and heads"))]
    DifferentInputLength {
        input: MatcherInput,
        location: Location,
    },
    /// occurs in step 1
    #[snafu(display("[{location}] Cannot find enough fitting pairs of outputs and heads"))]
    NotEnoughPairings {
        transfer: TransferOneTwo,
        location: Location,
    },
    /// occurs in step 2
    #[snafu(display("[{location}] Unable to find a single full cardinality matching"))]
    LowCardinality {
        transfer: TransferTwoThree,
        location: Location,
    },
}
