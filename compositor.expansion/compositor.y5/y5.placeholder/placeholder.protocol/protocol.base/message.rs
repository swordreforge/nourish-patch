use uuid::Uuid;
use compositor_introspection_launchplan_plan_base::LaunchPlan;

#[derive(Debug)]
pub struct PlaceholderMessage {
    pub uuid: Uuid,
    pub action: PlaceholderAction
}

#[derive(Debug)]
pub enum PlaceholderAction {
    Save(LaunchPlan),
    Erase(),
    Launch()
}
