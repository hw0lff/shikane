mod args;

use std::collections::VecDeque;

use clap::Parser;
#[allow(unused_imports)]
use log::{debug, error, info, trace, warn};
use snafu::{prelude::*, Location};

use crate::client::args::IncludeSearchFields;
use crate::daemon::profile_manager::ProfileManager;
use crate::error;
use crate::ipc::{IpcRequest, IpcResponse, IpcStream};
use crate::matching::MatchReport;
use crate::profile::{ConvertError, ConverterSettings, Profile};
use crate::wl_backend::WlHead;

pub use self::args::ShikaneCtl;

pub fn client(args: Option<ShikaneCtl>) {
    let args = match args {
        Some(args) => args,
        None => ShikaneCtl::parse(),
    };
    if let Err(err) = run(args) {
        error!("{}", error::report(err.as_ref()))
    }
}

fn run(args: ShikaneCtl) -> Result<(), Box<dyn snafu::Error>> {
    let mut ipc = match args.socket {
        Some(ref socket) => IpcStream::connect_to(socket)?,
        None => IpcStream::connect()?,
    };

    let request: IpcRequest = args.cmd.clone().into();
    ipc.send(&request)?;
    let response: IpcResponse = ipc.recv()?;
    trace!("{response:?}");

    match response {
        IpcResponse::CurrentHeads(heads) => print_current_configuration(args, heads)?,
        IpcResponse::Error(err) => error!("{err}"),
        IpcResponse::Generic(s) => println!("{s}"),
        IpcResponse::MatchReports(reports) => print_match_reports(reports),
        IpcResponse::Success => {}
    }

    Ok(())
}

fn print_current_configuration(
    args: ShikaneCtl,
    heads: VecDeque<WlHead>,
) -> Result<(), ClientError> {
    let (profile_name, search_fields) = match args.cmd {
        args::Command::Export(cmd_export) => {
            let sf = cmd_export
                .search_fields
                .unwrap_or(IncludeSearchFields::default())
                .into();
            (cmd_export.profile_name, sf)
        }
        _ => return CommandResponseMismatchCtx {}.fail(),
    };

    let settings = ConverterSettings::default()
        .profile_name(profile_name)
        .include_search_fields(search_fields)
        .converter()
        .run(heads)
        .context(ConvertCtx)?;
    println!("{settings}");
    Ok(())
}

fn print_match_reports(reports: VecDeque<MatchReport>) {
    let variants = ProfileManager::collect_variants_from_reports(&reports);
    let mut prev_profile: Option<Profile> = None;
    println!("total valid variants: {}", variants.len());
    for v in variants {
        match prev_profile {
            Some(ref profile) if *profile == v.profile => {}
            _ => {
                println!("{:?}", v.profile.name);
                prev_profile = Some(v.profile.clone());
            }
        }
        println!(
            "\t(specificity, deviation): ({}, {})",
            v.specificity(),
            v.mode_deviation()
        );
    }

    println!();
    println!("[report specific values]");
    for r in reports {
        println!("profile name: {:?}", r.profile.name);
        println!("\tunpaired heads: {}", r.unpaired_heads.len());
        println!("\tunpaired outputs: {}", r.unpaired_outputs.len());
        println!("\tunrelated pairings: {}", r.unrelated_pairings.len());
        println!("\tinvalid subsets: {}", r.invalid_subsets.len());
    }
}

#[derive(Debug, Snafu)]
#[snafu(context(suffix(Ctx)))]
pub enum ClientError {
    #[snafu(display("[{location}] Missing data from arguments for handling IPC response"))]
    CommandResponseMismatch { location: Location },
    #[snafu(display("[{location}] Cannot convert current output configuration to TOML"))]
    Convert {
        source: ConvertError,
        location: Location,
    },
}
