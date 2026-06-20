/// Where a hint came from. Used for debugging and for showing alternatives
/// in the UI ("this value was inferred from X; the runtime cmdline disagrees").
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct HintSource {
    pub method: SourceMethod,
    pub detail: String,
}

impl HintSource {
    pub fn new(method: SourceMethod, detail: impl Into<String>) -> Self {
        Self { method, detail: detail.into() }
    }
}

/// Broad categorization of where a hint's data was read from.
///
/// `DerivedFromConfig` covers anything synthesized from on-disk app state
/// (Chrome's Local State, JetBrains' recentProjects.xml, etc.) or computed
/// from already-captured data.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum SourceMethod {
    WaylandSurface,
    ProcExe,
    ProcCmdline,
    ProcEnviron,
    ProcCgroup,
    ProcTree,
    DesktopEntry,
    IconTheme,
    WindowTitle,
    DerivedFromConfig,
    UserProvided,
}

/// How much we trust a hint. The Draft initializer picks the highest-
/// confidence hint per attribute as the default value.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, serde::Serialize, serde::Deserialize)]
pub enum Confidence {
    Low,
    Medium,
    High,
}

impl Confidence {
    pub fn rank(self) -> u8 {
        match self {
            Confidence::High => 2,
            Confidence::Medium => 1,
            Confidence::Low => 0,
        }
    }
}
