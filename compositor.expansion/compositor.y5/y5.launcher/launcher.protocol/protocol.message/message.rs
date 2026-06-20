use std::path::PathBuf;
use compositor_monitor_launcher_ui_base::{Application, Direction};

#[derive(Debug)]
pub struct LauncherMessage {
    pub message: Source
}

#[derive(Debug)]
pub enum Source{
    Internal(InternalAction),
    External(ExternalAction)
}

#[derive(Debug)]
pub enum InternalAction {
    Start
}

#[derive(Debug)]
pub enum ExternalAction {
    Start{
        id: String,
        bin: PathBuf,
        args: Vec<String>,
        direction: Direction,
    },
    Exit,
}