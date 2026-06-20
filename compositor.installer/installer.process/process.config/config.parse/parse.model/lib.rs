//! Config model: the compositor environment (`Env`) and the prompted base
//! configuration (`BaseConfig`). Pure std.

/// The compositor's runtime configuration — mirrors the `Environment` struct the
/// compositor reads from its settings.json file. Every field is emitted (the
/// compositor requires all of them; no optionals, no defaults).
#[derive(Clone, Debug)]
pub struct Env {
    pub renderer: String,
    pub renderer_fallback: bool,
    pub renderer_sync: String,
    pub hdr: bool,
    pub depth: u8,
    pub vrr: bool,
    pub render_node: String,
    pub desktop_name: String,
    pub log_level: String,
    pub vk_diag: String,
    pub capture_encoder: String,
    pub window_client_size_fallback: bool,
    pub window_subsurface_shrinks: bool,
}

impl Env {
    /// Emit the settings JSON. Hand-rolled (no serde dep); string fields are
    /// escaped for `"` and `\`.
    pub fn to_json(&self) -> String {
        format!(
            "{{\"renderer\":\"{}\",\"renderer_fallback\":{},\"renderer_sync\":\"{}\",\
             \"hdr\":{},\"depth\":{},\"vrr\":{},\"render_node\":\"{}\",\
             \"desktop_name\":\"{}\",\"log_level\":\"{}\",\"vk_diag\":\"{}\",\
             \"capture_encoder\":\"{}\",\"window_client_size_fallback\":{},\
             \"window_subsurface_shrinks\":{}}}",
            esc(&self.renderer),
            self.renderer_fallback,
            esc(&self.renderer_sync),
            self.hdr,
            self.depth,
            self.vrr,
            esc(&self.render_node),
            esc(&self.desktop_name),
            esc(&self.log_level),
            esc(&self.vk_diag),
            esc(&self.capture_encoder),
            self.window_client_size_fallback,
            self.window_subsurface_shrinks,
        )
    }
}

fn esc(s: &str) -> String {
    s.replace('\\', "\\\\").replace('"', "\\\"")
}

/// Values prompted once for the default Y5 Desktop. Most propagate unchanged into
/// every other preset (the user enters the render node, log level, etc. once).
#[derive(Clone, Debug)]
pub struct BaseConfig {
    pub render_node: String,
    /// Root XDG desktop name; variants append a suffix (Dev, DevGles, …).
    pub desktop_name_root: String,
    pub log_level: String,
    /// Default scanout depth for the Y5 Desktop (10 per spec — same as Dev).
    pub depth: u8,
    pub vrr: bool,
    pub renderer_fallback: bool,
    /// Default sync mode for the Y5 Desktop / Dev presets: "" | "infence" | "kms".
    pub renderer_sync: String,
}

impl Default for BaseConfig {
    fn default() -> Self {
        BaseConfig {
            render_node: "/dev/dri/renderD128".to_string(),
            desktop_name_root: "Y5Compositor".to_string(),
            log_level: "info,warn,error".to_string(),
            depth: 10,
            vrr: true,
            renderer_fallback: false,
            renderer_sync: "infence".to_string(),
        }
    }
}
