mod comparator;
mod hopcroft_karp_map;
mod pairing;
mod pipelined;

use std::collections::VecDeque;

#[allow(unused_imports)]
use log::{debug, error, info, trace, warn};
use serde::{Deserialize, Serialize};

use crate::error;
use crate::pipeline::Pipeline;
use crate::profile::{Output, Profile};
use crate::variant::ValidVariant;
use crate::wl_backend::WlHead;

pub use self::comparator::{Comparator, ComparatorInfo};
pub use self::hopcroft_karp_map::{Edge, HopcroftKarpMap};
pub use self::pairing::{
    IntermediatePairing, IntermediatePairingWithMultipleModes, IntermediatePairingWithoutMode,
    Pairing, PairingWithMode, PairingWithoutMode, UnrelatedPairing,
};
use self::pipelined::{MatchPipelineError, MatcherOutput};

pub const MAX_RR_DEVIATION: i32 = 500;

pub struct ProfileMatcher;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct MatchReport {
    // input
    pub(crate) profile: Profile,
    wl_heads: VecDeque<WlHead>,

    // from stage 1
    pub(crate) unpaired_heads: Vec<WlHead>,
    pub(crate) unpaired_outputs: Vec<Output>,
    pub(crate) unrelated_pairings: Vec<UnrelatedPairing>,
    // from stage 2
    // contains lists of pairings where [pairing].len() != wl_heads.len()
    pub(crate) invalid_subsets: Vec<Vec<IntermediatePairing>>,
    // from stage 3
    pub(crate) valid_variants: VecDeque<ValidVariant>,
}

impl ProfileMatcher {
    pub fn create_report(profile: Profile, wl_heads: VecDeque<WlHead>) -> Option<MatchReport> {
        let p = Pipeline::new(pipelined::Stage1)
            .add_pipe(pipelined::Stage2)
            .add_pipe(pipelined::Stage3);

        let input = pipelined::MatcherInput::new(wl_heads, profile.clone());
        let result: Result<MatcherOutput, MatchPipelineError> = p.execute(input);

        match result {
            Ok(mo) => Some(mo.into_report()),
            Err(err) => {
                debug!(
                    "matching error with profile {:?}: {}",
                    profile.name,
                    error::report(&err)
                );
                if let MatchPipelineError::DifferentInputLength { ref input, .. } = err {
                    warn!("This should not have happened: {}", error::report(&err));
                    warn!("Please report this :)");
                    warn!("Occured with the following parameters: {input:?}");
                }
                None
            }
        }
    }
}
