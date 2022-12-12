use clap::Parser;
use std::path::PathBuf;

#[derive(Debug, Parser)]
#[clap(version)]
pub struct ShikaneArgs {
    /// Path to config file
    #[clap(short, long, value_name = "PATH")]
    pub config: Option<PathBuf>,

    /// Enable oneshot mode
    ///
    /// Exit after a profile has been applied or
    /// if no profile was matched
    #[clap(short, long)]
    pub oneshot: bool,

    /// Apply profiles untested
    #[clap(short, long, parse(try_from_str), default_value = "true", hide = true)]
    pub skip_tests: bool,
}
