use crate::profile::ShikaneProfilePlan;
use std::process::Command;

#[allow(unused_imports)]
use log::{debug, error, info, trace, warn};

pub fn execute_plan_commands(plan: &ShikaneProfilePlan, oneshot: bool) {
    let mut cmd_list = create_command_list(plan);
    if cmd_list.is_empty() {
        return;
    }

    trace!("Starting command exec thread");
    let handle = std::thread::Builder::new()
        .name("command exec".into())
        .spawn(move || {
            cmd_list.iter_mut().for_each(|(exec, cmd)| {
                execute_command(exec, cmd);
            });
        });

    if let Err(err) = handle {
        error!("Cannot spawn thread {:?}", err);
        return;
    }

    if !oneshot {
        return;
    }

    if let Err(err) = handle.unwrap().join() {
        error!("Cannot join thread {:?}", err)
    }
}

fn create_command_list(plan: &ShikaneProfilePlan) -> Vec<(String, Command)> {
    let mut cmd_list: Vec<(String, Command)> = vec![];
    let env_vars = vec![("SHIKANE_PROFILE_NAME", plan.profile.name.as_str())];
    assemble_commands(&mut cmd_list, plan.profile.exec.clone(), &env_vars);
    for (o, oh, _) in plan.config_set.iter() {
        let env_vars = vec![("SHIKANE_OUTPUT_NAME", oh.name.as_str())];
        assemble_commands(&mut cmd_list, o.exec.clone(), &env_vars);
    }
    cmd_list
}

fn assemble_commands(
    cmd_list: &mut Vec<(String, Command)>,
    vexec: Option<Vec<String>>,
    env_vars: &[(&str, &str)],
) {
    if let Some(exec) = vexec {
        for cmd in exec.iter() {
            let mut c = Command::new("sh");
            c.arg("-c").arg(cmd);
            c.envs(env_vars.to_owned());
            cmd_list.push((cmd.clone(), c));
        }
    }
}

fn execute_command(exec: &mut String, cmd: &mut Command) {
    debug!("[Cmd] {:?}", exec);
    let output = match cmd.output() {
        Ok(o) => o,
        Err(err) => {
            error!("failed to spawn command: {:?} {}", exec, err);
            return;
        }
    };
    if let Ok(stdout) = String::from_utf8(output.stdout) {
        trace!("[Out] {:?}", stdout)
    }
}
