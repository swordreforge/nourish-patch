//! Base types for the tile pyramid: rectangle, error, level metadata.
use serde::{Deserialize, Serialize};
use thiserror::Error;

/// A 2-D rectangle with f64 coordinates.
#[derive(Debug, Clone, Copy, Default)]
pub struct RectF64 {
    pub x: f64,
    pub y: f64,
    pub w: f64,
    pub h: f64,
}

impl RectF64 {
    pub const fn new(x: f64, y: f64, w: f64, h: f64) -> Self {
        Self { x, y, w, h }
    }
}

/// Errors during tile pyramid operations.
#[derive(Debug, Error)]
pub enum TileError {
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),
    #[error("image decode error: {0}")]
    Image(#[from] image::ImageError),
    #[error("JSON (de)serialization: {0}")]
    Json(#[from] serde_json::Error),
    #[error("source image too small (minimum {min}×{min} px)")]
    TooSmall { min: u32 },
    #[error("invalid LOD index {0}")]
    InvalidLod(u8),
}

/// One level-of-detail in the tile pyramid.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LevelMeta {
    pub level: u8,
    pub w: u32,
    pub h: u32,
    pub cols: u32,
    pub rows: u32,
}
