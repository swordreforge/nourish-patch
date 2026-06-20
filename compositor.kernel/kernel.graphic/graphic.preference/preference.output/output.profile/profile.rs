//! Typed per-monitor preference keyed by EDID identity. Self-contained value;
//! population out of scope.

#[derive(Debug, Clone)]
pub enum ModeRequest {
    /// Pick from advertised modes (current default policy applies when None).
    Advertised { width: u16, height: u16, refresh_mhz: u32 },
    /// Synthesize via CVT (requires the mode-synthesis safety enable).
    Cvt { width: u16, height: u16, refresh: f64 },
    /// Raw modeline string (requires the mode-synthesis safety enable).
    Modeline(String),
}

#[derive(Debug, Clone, Default)]
pub struct OutputProfile {
    /// EDID identity string this profile applies to ("make model serial").
    /// `None` = applies to any output (single-output era default).
    pub identity: Option<String>,
    pub mode: Option<ModeRequest>,
    pub position: (i32, i32),
    pub scale: Option<f64>,
}

pub fn get() -> Vec<OutputProfile> {
    Vec::new()
}
