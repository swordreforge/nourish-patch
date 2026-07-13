//! Tile index metadata and query methods (LOD selection, tile enumeration).
use std::path::PathBuf;

use serde::{Deserialize, Serialize};

pub use compositor_background_two_draw_tile_base::{LevelMeta, RectF64, TileError};

/// Full index describing a tile pyramid on disk.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TileIndex {
    pub source: PathBuf,
    pub source_w: u32,
    pub source_h: u32,
    pub tile_size: u32,
    pub levels: Vec<LevelMeta>,
}

impl TileIndex {
    pub fn max_level(&self) -> u8 {
        self.levels.len().saturating_sub(1) as u8
    }

    pub fn level(&self, lod: u8) -> Result<&LevelMeta, TileError> {
        self.levels.get(lod as usize).ok_or(TileError::InvalidLod(lod))
    }

    /// Get the pixel dimensions of a tile in the given LOD.
    pub fn tile_dimensions(&self, lod: u8, col: u32, row: u32) -> (u32, u32) {
        let lm = self.level(lod).unwrap_or_else(|_| &self.levels[0]);
        let ts = self.tile_size;
        let tw = if col < lm.cols.saturating_sub(1) { ts } else { lm.w - col * ts };
        let th = if row < lm.rows.saturating_sub(1) { ts } else { lm.h - row * ts };
        (tw.max(1), th.max(1))
    }

    /// World-space image bounds in the y5 coordinate model.
    pub const WORLD_W: f64 = 10.0;
    pub const WORLD_H: f64 = 2.0;

    /// Choose the LOD where one tile pixel ≈ one screen pixel.
    pub fn select_lod(&self, zoom: f64, screen_w: f64) -> u8 {
        let max = self.max_level();
        let world_w = self.source_w as f64;
        let viewport_world = screen_w / zoom;
        let visible_frac = (viewport_world / world_w).clamp(0.0, 1.0);
        let visible_source_px = screen_w / visible_frac.max(0.001);
        let lod_f = (self.source_w as f64 / visible_source_px).log2().floor();
        (lod_f as u8).clamp(0, max)
    }

    /// Compute the set of tile coordinates visible within `world_rect`.
    pub fn covering_tiles(&self, lod: u8, world_rect: &RectF64) -> Vec<(u8, u32, u32)> {
        let max = self.max_level();
        if lod > max {
            return vec![];
        }
        let lm = &self.levels[lod as usize];
        let ts = self.tile_size as f64;

        let x0 = (world_rect.x / Self::WORLD_W * lm.w as f64).max(0.0);
        let y0 = (world_rect.y / Self::WORLD_H * lm.h as f64).max(0.0);
        let x1 = ((world_rect.x + world_rect.w) / Self::WORLD_W * lm.w as f64).min(lm.w as f64);
        let y1 = ((world_rect.y + world_rect.h) / Self::WORLD_H * lm.h as f64).min(lm.h as f64);

        let col_start = (x0 / ts).floor() as u32;
        let col_end = ((x1 / ts).ceil() as u32).min(lm.cols.saturating_sub(1));
        let row_start = (y0 / ts).floor() as u32;
        let row_end = ((y1 / ts).ceil() as u32).min(lm.rows.saturating_sub(1));

        let mut tiles = Vec::with_capacity(
            ((col_end - col_start + 1) * (row_end - row_start + 1)) as usize,
        );
        for row in row_start..=row_end {
            for col in col_start..=col_end {
                tiles.push((lod, col, row));
            }
        }
        tiles
    }
}
