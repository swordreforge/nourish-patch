//! Hint value types + sandbox parsing (split out of window.base `hints`).

/// Value types used by built-in hint attributes.
pub mod values {
    /// An environment variable pair, used as the value type for the EnvOverlay
    /// attribute. Each captured env var becomes its own hint, so multiple
    /// EnvOverlay items coexist in InferredHints.
    #[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
    pub struct EnvPair {
        pub key: String,
        pub value: String,
    }

    /// Sandbox identity derived from cgroup / namespace inspection.
    #[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
    pub enum SandboxIdentity {
        None,
        Flatpak { app_id: String },
        Snap { instance_name: String },
        OtherContainer { hint: String },
    }
}

/// `/proc/<pid>/cgroup` parsing into a sandbox identity.
pub mod sandbox {
    use crate::values::SandboxIdentity;

    /// Recognized patterns:
    /// - `app-flatpak-<id>-<pid>.scope` → Flatpak
    /// - `snap.<name>.<command>.<uuid>.scope` → Snap
    /// - anything else → None
    pub fn parse_sandbox(cgroup_line: &str) -> SandboxIdentity {
        for segment in cgroup_line.split('/') {
            if let Some(rest) = segment.strip_prefix("app-flatpak-") {
                if let Some(id_end) = rest.rfind('-') {
                    let id = &rest[..id_end];
                    return SandboxIdentity::Flatpak {
                        app_id: id.to_string(),
                    };
                }
            }
            if let Some(rest) = segment.strip_prefix("snap.") {
                if let Some(name_end) = rest.find('.') {
                    let name = &rest[..name_end];
                    return SandboxIdentity::Snap {
                        instance_name: name.to_string(),
                    };
                }
            }
        }
        SandboxIdentity::None
    }
}
