use thiserror;

#[derive(Debug, thiserror::Error)]
pub enum EngineInitError {
    #[error("Bevy app build failed: {0}")]
    AppBuild(String),
}
