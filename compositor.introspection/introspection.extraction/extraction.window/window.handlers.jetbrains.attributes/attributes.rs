use compositor_introspection_extraction_window_handlers_jetbrains_id::id::id;
use compositor_introspection_extraction_window_hints_attribute::attribute::HintAttribute;
use compositor_introspection_extraction_window_hints_descriptor::descriptor::{AttributeDescriptor, AttributeKind};
use compositor_introspection_extraction_window_hints_id::category::AttributeCategory;
use std::path::PathBuf;

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum Product {
    IntelliJIDEA,
    PyCharm,
    GoLand,
    CLion,
    WebStorm,
    Rider,
    RubyMine,
    PhpStorm,
    DataGrip,
    Unknown(String),
}

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum LauncherKind {
    Toolbox,
    SystemPackage,
    Flatpak,
    Snap,
    Unknown,
}

#[derive(Debug)]
pub struct ProductAttr;
impl HintAttribute for ProductAttr {
    type Value = Product;
    fn name() -> &'static str { "jetbrains.product" }
    fn category() -> AttributeCategory { AttributeCategory::HandlerScoped(id()) }
    fn descriptor() -> AttributeDescriptor {
        AttributeDescriptor::new(
            Self::name(), "Product", Self::category(),
            AttributeKind::EnumOf(vec![
                "IntelliJ IDEA", "PyCharm", "GoLand", "CLion", "WebStorm",
                "Rider", "RubyMine", "PhpStorm", "DataGrip",
            ]),
        )
    }
}

#[derive(Debug)]
pub struct LauncherKindAttr;
impl HintAttribute for LauncherKindAttr {
    type Value = LauncherKind;
    fn name() -> &'static str { "jetbrains.launcher_kind" }
    fn category() -> AttributeCategory { AttributeCategory::HandlerScoped(id()) }
    fn descriptor() -> AttributeDescriptor {
        AttributeDescriptor::new(Self::name(), "Launcher kind", Self::category(), AttributeKind::EnumOf(vec!["Toolbox", "SystemPackage", "Flatpak", "Snap"]))
    }
}

#[derive(Debug)]
pub struct ProjectNameGuess;
impl HintAttribute for ProjectNameGuess {
    type Value = String;
    fn name() -> &'static str { "jetbrains.project_name_guess" }
    fn category() -> AttributeCategory { AttributeCategory::HandlerScoped(id()) }
    fn descriptor() -> AttributeDescriptor {
        AttributeDescriptor::new(Self::name(), "Project name (guess)", Self::category(), AttributeKind::Text)
    }
}

#[derive(Debug)]
pub struct ProjectPath;
impl HintAttribute for ProjectPath {
    type Value = PathBuf;
    fn name() -> &'static str { "jetbrains.project_path" }
    fn category() -> AttributeCategory { AttributeCategory::HandlerScoped(id()) }
    fn descriptor() -> AttributeDescriptor {
        AttributeDescriptor::new(Self::name(), "Project path", Self::category(), AttributeKind::Path)
    }
}

/// All JetBrains-scoped attribute descriptors, in display order.
pub fn descriptors() -> Vec<AttributeDescriptor> {
    vec![
        ProductAttr::descriptor(),
        LauncherKindAttr::descriptor(),
        ProjectNameGuess::descriptor(),
        ProjectPath::descriptor(),
    ]
}
