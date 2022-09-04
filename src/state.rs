use std::ops::Not;

use crate::backend::ShikaneBackend;
use crate::config::Profile;
use crate::config::ShikaneConfig;

#[allow(unused_imports)]
use log::{debug, error, info, trace, warn};

#[derive(Debug)]
pub(crate) struct ShikaneState {
    pub(crate) first_done: bool,
    pub(crate) backend: ShikaneBackend,
    pub(crate) config: ShikaneConfig,
}

impl ShikaneState {
    pub(crate) fn new(backend: ShikaneBackend, config: ShikaneConfig) -> Self {
        Self {
            first_done: Default::default(),
            backend,
            config,
        }
    }

    pub(crate) fn configure(&mut self) {
        trace!("[Configure] triggered");
        let profile = self
            .config
            .profiles
            .iter()
            .find(|profile| self.match_profile(profile))
            .cloned();

        match profile {
            Some(profile) => self.configure_profile(&profile),
            None => {
                warn!("[Configure] No profile matched")
            }
        }
    }

    fn match_profile(&self, profile: &Profile) -> bool {
        if profile.outputs.len() != self.backend.output_heads.len() {
            return false;
        }

        let mut unmatched_heads: Vec<&_> = self.backend.output_heads.values().into_iter().collect();
        // This code removes an `OutputHead` from `unmatched_heads` if it matches against the pattern in `output.r#match`
        profile.outputs.iter().for_each(|output| {
            unmatched_heads = unmatched_heads
                .iter()
                .filter_map(|head| head.matches(&output.r#match).not().then_some(*head))
                .collect();
        });
        // If the list of unmatched heads is empty then all heads have been matched against the provided outputs
        unmatched_heads.is_empty()
    }

    fn configure_profile(&mut self, profile: &Profile) {
        let opc = self.backend.create_configuration();
        debug!("Configuring profile: {}", profile.name);

        profile.outputs.iter().for_each(|output| {
            let (head_id, output_head) = self.backend.match_head(&output.r#match).unwrap();
            trace!("Setting Head: {:?}", output_head.name);
            let (mode_id, output_mode) = self.backend.match_mode(head_id, &output.mode).unwrap();
            trace!("Setting Mode: {:?}", output_mode);
            let head = self.backend.head_from_id(head_id.clone());
            let mode = self.backend.mode_from_id(mode_id);

            if output.enable {
                let opch = opc.enable_head(&head, &self.backend.qh, self.backend.data);
                opch.set_mode(&mode);
                opch.set_position(output.position.x, output.position.y);
            } else {
                opc.disable_head(&head);
            }
        });

        opc.apply();
    }

    pub(crate) fn idle(&mut self) {
        self.backend.refresh();
    }
}
