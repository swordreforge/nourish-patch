//! Top-level spawn entry points: plain exec or systemd-run wrapped.

use std::ffi::OsStr;
use std::io;
use std::path::Path;
use std::process::Command;

use compositor_introspection_launchplan_plan_exec_opts::exec::SystemdRunOpts;
use compositor_introspection_launchplan_plan_exec_run::exec::wrap_and_execute;

pub fn spawn_via_exec<S, I>(
    bin: &Path,
    args: I,
    extra_env: &[(String, String)],
    unit_name: &str,
) -> io::Result<()>
where
    I: IntoIterator<Item = S>,
    S: AsRef<OsStr>,
{
    let mut cmd = Command::new(bin);
    cmd.args(args);
    for (k, v) in extra_env {
        cmd.env(k, v);
    }

    cmd.spawn().map(|a| ())
}

pub fn spawn_via_systemd<S, I>(
    bin: &Path,
    args: I,
    extra_env: &[(String, String)],
    unit_name: &str,
) -> io::Result<()>
where
    I: IntoIterator<Item = S>,
    S: AsRef<OsStr>,
{
    // Build a plain Command describing what we want to run.
    // wrap_and_execute reads program, args, env, and current_dir from it.
    let mut cmd = Command::new(bin);
    cmd.args(args);
    for (k, v) in extra_env {
        cmd.env(k, v);
    }

    // This one doesn't need a PID at all: hand it to the systemd-run
    // wrapper with defaults and a zero PID-poll budget.
    let opts = SystemdRunOpts::new_detach(unit_name);
    let res = wrap_and_execute(&cmd, &opts);

    if !res.is_ok() {
        return Err(io::Error::new(
            io::ErrorKind::Other,
            format!("Error starting process through systemd."),
        ));
    }
    Ok(())
}
