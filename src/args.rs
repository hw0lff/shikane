use clap::Parser;
use std::path::PathBuf;

#[derive(Debug, Parser)]
#[clap(version)]
pub(crate) struct ShikaneArgs {
    /// Path to config file
    #[clap(short, long, value_name = "PATH")]
    pub(crate) config: Option<PathBuf>,

    /// Enable oneshot mode
    ///
    /// Exit after a profile has been applied or
    /// if no profile was matched
    #[clap(short, long)]
    pub(crate) oneshot: bool,
}
