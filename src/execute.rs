use std::collections::HashMap;
use std::process::Command;

#[allow(unused_imports)]
use log::{debug, error, info, trace, warn};

#[derive(Clone, Debug)]
pub struct CommandBuilder {
    profile_name: String,
    profile_commands: Vec<String>,
    heads: HashMap<String, Vec<String>>,
    oneshot: bool,
}

impl CommandBuilder {
    pub fn new(profile_name: String) -> Self {
        Self {
            profile_name,
            profile_commands: Default::default(),
            heads: Default::default(),
            oneshot: false,
        }
    }
    pub fn oneshot(&mut self, oneshot: bool) {
        self.oneshot = oneshot
    }
    pub fn profile_commands(&mut self, profile_commands: Vec<String>) {
        self.profile_commands = profile_commands
    }
    pub fn insert_head_commands(&mut self, head_name: String, output_commands: Vec<String>) {
        self.heads.insert(head_name, output_commands);
    }

    pub fn execute(self) {
        let oneshot = self.oneshot;
        let mut cmd_list = self.create_command_list();
        if cmd_list.is_empty() {
            return;
        }

        trace!("starting command executer thread");
        let handle = std::thread::Builder::new()
            .name("command exec".into())
            .spawn(move || {
                cmd_list.iter_mut().for_each(|(exec, cmd)| {
                    execute_command(exec, cmd);
                });
            });

        if let Err(err) = handle {
            error!("cannot spawn thread {:?}", err);
            return;
        }

        // return immediately if we are running as a daemon
        if !oneshot {
            return;
        }

        if let Err(err) = handle.unwrap().join() {
            error!("cannot join thread {:?}", err)
        }
    }

    fn create_command_list(self) -> Vec<(String, Command)> {
        let mut cmd_list: Vec<(String, Command)> = vec![];
        let env_vars = vec![("SHIKANE_PROFILE_NAME", self.profile_name.as_str())];
        assemble_commands(&mut cmd_list, self.profile_commands, &env_vars);
        for (head_name, output_commands) in self.heads {
            let env_vars = vec![("SHIKANE_OUTPUT_NAME", head_name.as_str())];
            assemble_commands(&mut cmd_list, output_commands, &env_vars);
        }
        cmd_list
    }
}

fn assemble_commands(
    cmd_list: &mut Vec<(String, Command)>,
    vexec: Vec<String>,
    env_vars: &[(&str, &str)],
) {
    for cmd in vexec {
        let mut c = Command::new("sh");
        c.arg("-c").arg(cmd.clone());
        c.envs(env_vars.to_owned());
        cmd_list.push((cmd, c));
    }
}

fn execute_command(exec: &mut String, cmd: &mut Command) {
    debug!("[cmd] {:?}", exec);
    let output = match cmd.output() {
        Ok(o) => o,
        Err(err) => {
            error!("failed to spawn command: {:?} {}", exec, err);
            return;
        }
    };
    if let Ok(stdout) = String::from_utf8(output.stdout) {
        trace!("[stdout] {:?}", stdout)
    }
    if let Ok(stderr) = String::from_utf8(output.stderr) {
        trace!("[stderr] {:?}", stderr)
    }
}
