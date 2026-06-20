use compositor_introspection_extraction_window_handlers_chrome_id::id::id;
use compositor_introspection_extraction_window_hints_attribute::attribute::HintAttribute;
use compositor_introspection_extraction_window_hints_descriptor::descriptor::{AttributeDescriptor, AttributeKind};
use compositor_introspection_extraction_window_hints_id::category::AttributeCategory;
use std::path::PathBuf;

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum BrowserVariant {
    Chrome,
    Chromium,
    Brave,
    Edge,
    Vivaldi,
    Other(String),
}

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct ChromeProfileInfo {
    pub directory_name: String,
    pub display_name: String,
}

#[derive(Debug)]
pub struct Variant;
impl HintAttribute for Variant {
    type Value = BrowserVariant;
    fn name() -> &'static str { "chrome.variant" }
    fn category() -> AttributeCategory { AttributeCategory::HandlerScoped(id()) }
    fn descriptor() -> AttributeDescriptor {
        AttributeDescriptor::new(Self::name(), "Variant", Self::category(), AttributeKind::EnumOf(vec!["Chrome", "Chromium", "Brave", "Edge", "Vivaldi"]))
    }
}

#[derive(Debug)]
pub struct ProfileDirectory;
impl HintAttribute for ProfileDirectory {
    type Value = String;
    fn name() -> &'static str { "chrome.profile_directory" }
    fn category() -> AttributeCategory { AttributeCategory::HandlerScoped(id()) }
    fn descriptor() -> AttributeDescriptor {
        AttributeDescriptor::new(Self::name(), "Profile directory", Self::category(), AttributeKind::Custom("chrome_profile"))
    }
}

#[derive(Debug)]
pub struct UserDataDir;
impl HintAttribute for UserDataDir {
    type Value = PathBuf;
    fn name() -> &'static str { "chrome.user_data_dir" }
    fn category() -> AttributeCategory { AttributeCategory::HandlerScoped(id()) }
    fn descriptor() -> AttributeDescriptor {
        AttributeDescriptor::new(Self::name(), "User-data directory", Self::category(), AttributeKind::Path)
    }
}

#[derive(Debug)]
pub struct AvailableProfiles;
impl HintAttribute for AvailableProfiles {
    type Value = Vec<ChromeProfileInfo>;
    fn name() -> &'static str { "chrome.available_profiles" }
    fn category() -> AttributeCategory { AttributeCategory::HandlerScoped(id()) }
    fn descriptor() -> AttributeDescriptor {
        AttributeDescriptor::new(Self::name(), "Available profiles", Self::category(), AttributeKind::Custom("chrome_profile_list"))
    }
}
