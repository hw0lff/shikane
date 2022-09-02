use clap::Parser;
use std::path::PathBuf;

#[derive(Parser)]
#[clap(version)]
pub(crate) struct ShikaneArgs {
    /// Path to config file
    #[clap(short, long, value_name = "PATH")]
    pub(crate) config: Option<PathBuf>,
}
