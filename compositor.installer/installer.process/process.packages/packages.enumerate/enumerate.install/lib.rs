//! The `dnf install` runner. Pure std.

use std::process::Command;

/// Install the given packages with dnf. With `dry_run`, the command is printed
/// only. Returns Err on a non-zero dnf exit.
pub fn dnf_install(packages: &[String], dry_run: bool) -> Result<(), String> {
    if packages.is_empty() {
        return Ok(());
    }
    let mut argv: Vec<String> = vec!["dnf".into(), "install".into(), "-y".into()];
    argv.extend(packages.iter().cloned());

    if dry_run {
        println!("  [dry-run] sudo {}", argv.join(" "));
        return Ok(());
    }

    let status = Command::new("sudo")
        .args(&argv)
        .status()
        .map_err(|e| format!("failed to run sudo dnf: {e}"))?;
    if status.success() {
        Ok(())
    } else {
        Err(format!("dnf install exited with {status}"))
    }
}
