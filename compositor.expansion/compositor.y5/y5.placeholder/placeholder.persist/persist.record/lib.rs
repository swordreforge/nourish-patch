// The `world.placeholder` table: each world's placeholders saved as the slim
// "prior data" needed to re-show, edit, and relaunch them — the launch plan's
// resolved fields, NOT the live process tree / pid / session plan. On load each
// record rebuilds a LaunchPlan (the saved values re-pushed as hints) so the
// placeholder reappears as an editable saved launcher under the same UUID.
pub mod base;
