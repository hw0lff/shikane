use clap::Subcommand;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Deserialize, Serialize, Subcommand)]
pub(crate) enum ShikaneCommand {
    Debug,
}
