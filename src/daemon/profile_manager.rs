use std::collections::{HashSet, VecDeque};

#[allow(unused_imports)]
use log::{debug, error, info, trace, warn};

use crate::matching::{MatchReport, ProfileMatcher};
use crate::profile::Profile;
use crate::search::SearchPattern;
use crate::variant::ValidVariant;
use crate::wl_backend::{LessEqWlHead, WlHead};

#[derive(Debug)]
pub struct ProfileManager {
    profiles: VecDeque<Profile>,
    variants: VecDeque<ValidVariant>,
    reports: VecDeque<MatchReport>,
    restriction: Option<Restriction>,
    cached_heads: VecDeque<WlHead>,
}

#[derive(Clone, Debug)]
pub struct Restriction {
    pattern: SearchPattern,
}

impl ProfileManager {
    pub fn new(profiles: VecDeque<Profile>) -> Self {
        Self {
            profiles,
            variants: Default::default(),
            reports: Default::default(),
            restriction: Default::default(),
            cached_heads: Default::default(),
        }
    }
    pub fn set_profiles(&mut self, profiles: VecDeque<Profile>) {
        self.profiles = profiles
    }
    pub fn reports(&self) -> &VecDeque<MatchReport> {
        &self.reports
    }

    pub fn next_variant(&mut self) -> Option<ValidVariant> {
        self.variants.pop_front()
    }
    /// Restrict profile selection to a profile with a name that matches the restriction.
    pub fn restrict(&mut self, rest: Restriction) {
        self.restriction = Some(rest)
    }
    /// Lift a previously set restriction.
    pub fn lift_restriction(&mut self) -> Option<Restriction> {
        self.restriction.take()
    }
    pub fn test_restriction(&self, rest: &Restriction) -> bool {
        self.profiles
            .iter()
            .any(|p| rest.pattern.matches(&p.name).0)
    }

    /// Delete old variants and reports
    pub fn clear(&mut self) {
        self.variants.clear();
        self.reports.clear();
        self.clear_cached_heads();
    }
    fn restricted_profiles(&self) -> VecDeque<Profile> {
        if let Some(ref rest) = self.restriction {
            return self
                .profiles
                .iter()
                .filter(|p| rest.pattern.matches(&p.name).0)
                .cloned()
                .collect();
        }
        self.profiles.clone()
    }
    pub fn generate_variants(&mut self, wl_heads: VecDeque<WlHead>) {
        self.cached_heads.clone_from(&wl_heads);
        let profiles: VecDeque<Profile> = self.restricted_profiles();
        self.lift_restriction();

        for profile in profiles {
            let (len_heads, len_outputs) = (wl_heads.len(), profile.outputs.len());
            if len_heads != len_outputs {
                continue;
            }
            debug!("len(outputs)={} len(heads)={}", len_outputs, len_heads);
            debug!("profile.name={}", profile.name);
            if let Some(report) = ProfileMatcher::create_report(profile, wl_heads.clone()) {
                let (len_variants, pname) = (report.valid_variants.len(), &report.profile.name);
                info!("len(valid variants, profile {:?})={}", pname, len_variants);
                self.reports.push_back(report);
            }
        }
        self.variants = Self::collect_variants_from_reports(&self.reports);
    }

    pub fn collect_variants_from_reports(
        reports: &VecDeque<MatchReport>,
    ) -> VecDeque<ValidVariant> {
        let mut variants: VecDeque<_> = reports
            .iter()
            .flat_map(|r| r.valid_variants.clone())
            .collect();
        variants.make_contiguous().sort_by(|a, b| {
            // sort specificity decreasingly and deviation increasingly
            (b.specificity(), a.mode_deviation()).cmp(&(a.specificity(), b.mode_deviation()))
        });

        trace!("printing specificity and deviation of sorted valid variants");
        variants.iter().for_each(|v| {
            trace!(
                "{}:(specificity, deviation):({}, {})",
                v.idx_str(),
                v.specificity(),
                v.mode_deviation()
            )
        });
        let (n, l) = (reports.len(), variants.len());
        debug!("len(total valid variants over {n} reports)={l}",);
        variants
    }

    pub fn is_cache_outdated(&self, wl_heads: &VecDeque<WlHead>) -> bool {
        let le_head_a: HashSet<LessEqWlHead> = self.cached_heads.iter().map(LessEqWlHead).collect();
        let le_head_b: HashSet<LessEqWlHead> = wl_heads.iter().map(LessEqWlHead).collect();
        le_head_a != le_head_b
    }

    pub fn clear_cached_heads(&mut self) {
        self.cached_heads.clear()
    }
}

impl From<SearchPattern> for Restriction {
    fn from(pattern: SearchPattern) -> Self {
        Self { pattern }
    }
}
impl From<regex::Regex> for Restriction {
    fn from(r: regex::Regex) -> Self {
        Self { pattern: r.into() }
    }
}
