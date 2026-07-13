//! Tile pyramid: metadata, filesystem I/O, LOD selection, tile enumeration.
//!
//! Pure-data layer — no smithay / GPU types. See `draw.wallpaper` for the
//! rendering path that consumes these tiles.

mod tile;
pub use tile::*;
