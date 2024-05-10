use std::path::PathBuf;

use clap::{Args, Parser, Subcommand};
#[allow(unused_imports)]
use log::{debug, error, info, trace, warn};

use crate::ipc::IpcRequest;
use crate::search::SearchField;

#[derive(Clone, Debug, Parser)]
#[command(version)]
pub struct ShikaneCtl {
    #[command(subcommand)]
    pub(super) cmd: Command,

    /// Connect to the specified socket
    #[clap(long, short)]
    pub(super) socket: Option<PathBuf>,
}

#[derive(Clone, Debug, Subcommand)]
pub enum Command {
    #[clap(subcommand, alias = "dbg", hide = true)]
    Debug(CmdDebug),
    Switch(CmdSwitch),
    Reload(CmdReload),
    Export(CmdExport),
}

/// Subcommand for debugging shikane and its configuration.
/// This interface is not stable and could change any time.
#[derive(Clone, Debug, Subcommand)]
pub enum CmdDebug {
    /// Print the current state of the daemon state machine
    CurrentState,
    /// List all valid variants and report data
    ListReports,
}

/// Use the given profile temporarily
#[derive(Clone, Debug, Args)]
pub struct CmdSwitch {
    /// Name of the profile
    name: String,
}

/// Reload the configuration file
#[derive(Clone, Debug, Args)]
pub struct CmdReload {
    /// Use this file instead of the current config file
    file: Option<PathBuf>,
}

/// Export the current display setup as shikane config.
/// Include vendor, model and serial number in the searches by default.
#[derive(Clone, Debug, Args)]
pub struct CmdExport {
    #[command(flatten)]
    pub search_fields: Option<IncludeSearchFields>,

    /// Name of the exported profile
    pub profile_name: String,
}

#[derive(Clone, Debug, Parser)]
#[clap(group = clap::ArgGroup::new("include_search_fields").multiple(true))]
pub struct IncludeSearchFields {
    /// Include the description in the searches
    #[arg(short, long, group = "include_search_fields")]
    description: bool,
    /// Include the name in the searches
    #[arg(short, long, group = "include_search_fields")]
    name: bool,
    /// Include the model in the searches
    #[arg(short, long, group = "include_search_fields")]
    model: bool,
    /// Include the serial number in the searches
    #[arg(short, long, group = "include_search_fields")]
    serial: bool,
    /// Include the vendor in the searches
    #[arg(short, long, group = "include_search_fields")]
    vendor: bool,
}

impl Default for IncludeSearchFields {
    fn default() -> Self {
        Self {
            description: false,
            name: false,
            model: true,
            serial: true,
            vendor: true,
        }
    }
}

impl From<IncludeSearchFields> for Vec<SearchField> {
    fn from(value: IncludeSearchFields) -> Self {
        let mut sf = vec![];
        if value.description {
            sf.push(SearchField::Description);
        }
        if value.name {
            sf.push(SearchField::Name);
        }
        if value.model {
            sf.push(SearchField::Model);
        }
        if value.serial {
            sf.push(SearchField::Serial);
        }
        if value.vendor {
            sf.push(SearchField::Vendor);
        }
        sf
    }
}

impl From<Command> for IpcRequest {
    fn from(cmd: Command) -> Self {
        match cmd {
            Command::Debug(c) => c.into(),
            Command::Switch(c) => Self::SwitchProfile(c.name),
            Command::Reload(c) => Self::ReloadConfig(c.file),
            Command::Export(_) => Self::CurrentHeads,
        }
    }
}

impl From<CmdDebug> for IpcRequest {
    fn from(value: CmdDebug) -> Self {
        match value {
            CmdDebug::CurrentState => Self::CurrentState,
            CmdDebug::ListReports => Self::MatchReports,
        }
    }
}
