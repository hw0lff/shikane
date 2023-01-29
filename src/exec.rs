use crate::profile::Profile;

#[allow(unused_imports)]
use log::{debug, error, info, trace, warn};

pub fn execute_profile_commands(profile: &Profile, oneshot: bool) {
    if let Some(exec) = &profile.exec {
        let exec = exec.clone();
        trace!("Starting command exec thread");
        let handle = match std::thread::Builder::new()
            .name("command exec".into())
            .spawn(move || {
                exec.iter().for_each(|cmd| execute_command(cmd));
            }) {
            Ok(joinhandle) => Some(joinhandle),
            Err(err) => {
                error!("cannot spawn thread {:?}", err);
                None
            }
        };

        if !oneshot {
            return;
        }
        if let Some(handle) = handle {
            match handle.join() {
                Ok(_) => {}
                Err(err) => {
                    error!("cannot join thread {:?}", err);
                }
            };
        }
    }
}

fn execute_command(cmd: &str) {
    use std::process::Command;
    if cmd.is_empty() {
        return;
    }
    debug!("[Cmd] {:?}", cmd);
    match Command::new("sh").arg("-c").arg(cmd).output() {
        Ok(output) => {
            if let Ok(stdout) = String::from_utf8(output.stdout) {
                trace!("[Out] {:?}", stdout)
            }
        }
        Err(_) => error!("failed to spawn command: {:?}", cmd),
    }
}
