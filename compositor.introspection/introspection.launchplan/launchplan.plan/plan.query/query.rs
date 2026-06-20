use std::collections::HashMap;
use std::io;
use std::process::{Command, Stdio};
use compositor_introspection_extraction_window_base::attributes::{DetectedHandler, EnvOverlay, ExecArgs, ExecProgram, WorkingDirectory};
use compositor_introspection_extraction_window_base::{AttributeCategory, HandlerId, HintAttribute};
use compositor_introspection_inference_hint_base::ApplicationData;
use compositor_introspection_launchplan_plan_preferences::Preferences;

pub fn initial_handler(application_data: &ApplicationData) -> Option<HandlerId> {
    application_data.best_value::<DetectedHandler>()
}

/// Effective value: preferences over inferred best; disabled => `None`.
pub fn current<A: HintAttribute>(
    global_preferences: &Preferences,
    handler_preferences: &HashMap<HandlerId, Preferences>,
    active_handler: Option<HandlerId>,
    application_data: &ApplicationData,
) -> Option<A::Value> {
    match A::category() {
        AttributeCategory::Identity | AttributeCategory::Launch => {
            if !global_preferences.is_enabled::<A>() { return None; }
            global_preferences.get::<A>().or_else(|| application_data.best_value::<A>())
        }
        AttributeCategory::HandlerScoped(handler_id) => {
            if active_handler != Some(handler_id) { return None; }
            let prefs = handler_preferences.get(&handler_id);
            let enabled = prefs.map(|p| p.is_enabled::<A>()).unwrap_or(true);
            if !enabled { return None; }
            prefs.and_then(|p| p.get::<A>()).or_else(|| application_data.best_value::<A>())
        }
    }
}

pub fn prefs_for<'a>(
    global_preferences: &'a Preferences,
    handler_preferences: &'a HashMap<HandlerId, Preferences>,
    active_handler: Option<HandlerId>,
    category: &AttributeCategory,
) -> Option<&'a Preferences> {
    match category {
        AttributeCategory::Identity | AttributeCategory::Launch => Some(global_preferences),
        AttributeCategory::HandlerScoped(handler_id) => {
            if active_handler != Some(*handler_id) { return None; }
            handler_preferences.get(handler_id)
        }
    }
}

pub fn prefs_for_mut<'a>(
    global_preferences: &'a mut Preferences,
    handler_preferences: &'a mut HashMap<HandlerId, Preferences>,
    category: &AttributeCategory,
) -> &'a mut Preferences {
    match category {
        AttributeCategory::Identity | AttributeCategory::Launch => global_preferences,
        AttributeCategory::HandlerScoped(handler_id) => {
            handler_preferences.entry(*handler_id).or_insert_with(Preferences::new)
        }
    }
}

/// Generic exec from program + args; `NotFound` if no program resolvable.
pub fn generic_command(
    program: Option<<ExecProgram as HintAttribute>::Value>,
    args: Option<<ExecArgs as HintAttribute>::Value>,
) -> Result<Command, io::Error> {
    let program = program
        .ok_or_else(|| io::Error::new(io::ErrorKind::NotFound, "launch plan has no program"))?;
    let mut cmd = Command::new(program);
    cmd.args(args.unwrap_or_default());
    Ok(cmd)
}

/// Apply working dir + env to `cmd`, inherit stdio, spawn, return the PID.
pub fn run_command(
    mut cmd: Command,
    working_dir: Option<<WorkingDirectory as HintAttribute>::Value>,
    env_pairs: Option<<EnvOverlay as HintAttribute>::Value>,
    extra_env: &[(String, String)],
) -> Result<Option<u32>, io::Error> {
    if let Some(wd) = working_dir {
        cmd.current_dir(wd);
    }
    for pair in env_pairs.unwrap_or_default() {
        cmd.env(pair.key, pair.value);
    }
    for (k, v) in extra_env {
        cmd.env(k, v);
    }
    cmd.stdin(Stdio::inherit());
    cmd.stdout(Stdio::inherit());
    cmd.stderr(Stdio::inherit());
    cmd.spawn().map(|w| Some(w.id()))
}
