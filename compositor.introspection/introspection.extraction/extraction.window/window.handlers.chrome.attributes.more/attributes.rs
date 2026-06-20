use compositor_introspection_extraction_window_handlers_chrome_attributes::attributes::{
    AvailableProfiles, ProfileDirectory, UserDataDir, Variant,
};
use compositor_introspection_extraction_window_handlers_chrome_id::id::id;
use compositor_introspection_extraction_window_hints_attribute::attribute::HintAttribute;
use compositor_introspection_extraction_window_hints_descriptor::descriptor::{AttributeDescriptor, AttributeKind};
use compositor_introspection_extraction_window_hints_id::category::AttributeCategory;

#[derive(Debug)]
pub struct ActiveTabTitleGuess;
impl HintAttribute for ActiveTabTitleGuess {
    type Value = String;
    fn name() -> &'static str { "chrome.active_tab_title_guess" }
    fn category() -> AttributeCategory { AttributeCategory::HandlerScoped(id()) }
    fn descriptor() -> AttributeDescriptor {
        AttributeDescriptor::new(Self::name(), "Active tab (guess)", Self::category(), AttributeKind::Text)
    }
}

#[derive(Debug)]
pub struct Urls;
impl HintAttribute for Urls {
    type Value = Vec<String>;
    fn name() -> &'static str { "chrome.urls" }
    fn category() -> AttributeCategory { AttributeCategory::HandlerScoped(id()) }
    fn descriptor() -> AttributeDescriptor {
        AttributeDescriptor::new(Self::name(), "URLs", Self::category(), AttributeKind::StringList)
    }
}

#[derive(Debug)]
pub struct AppModeUrl;
impl HintAttribute for AppModeUrl {
    type Value = String;
    fn name() -> &'static str { "chrome.app_mode_url" }
    fn category() -> AttributeCategory { AttributeCategory::HandlerScoped(id()) }
    fn descriptor() -> AttributeDescriptor {
        AttributeDescriptor::new(Self::name(), "App-mode URL", Self::category(), AttributeKind::Text)
    }
}

/// Whether to launch with `--new-window`. No inference; defaults to true
/// in the synthesizer if no preference is set.
#[derive(Debug)]
pub struct NewWindow;
impl HintAttribute for NewWindow {
    type Value = bool;
    fn name() -> &'static str { "chrome.new_window" }
    fn category() -> AttributeCategory { AttributeCategory::HandlerScoped(id()) }
    fn descriptor() -> AttributeDescriptor {
        AttributeDescriptor::new(Self::name(), "New window", Self::category(), AttributeKind::Bool)
    }
}

/// Whether to launch with `--incognito`. No inference; defaults to false
/// in the synthesizer.
#[derive(Debug)]
pub struct Incognito;
impl HintAttribute for Incognito {
    type Value = bool;
    fn name() -> &'static str { "chrome.incognito" }
    fn category() -> AttributeCategory { AttributeCategory::HandlerScoped(id()) }
    fn descriptor() -> AttributeDescriptor {
        AttributeDescriptor::new(Self::name(), "Incognito", Self::category(), AttributeKind::Bool)
    }
}

/// All Chrome-scoped attribute descriptors, in display order.
pub fn descriptors() -> Vec<AttributeDescriptor> {
    vec![
        Variant::descriptor(),
        ProfileDirectory::descriptor(),
        UserDataDir::descriptor(),
        AvailableProfiles::descriptor(),
        ActiveTabTitleGuess::descriptor(),
        Urls::descriptor(),
        AppModeUrl::descriptor(),
        NewWindow::descriptor(),
        Incognito::descriptor(),
    ]
}
