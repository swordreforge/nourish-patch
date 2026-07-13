//! Tile pyramid — facade re-exporting sub-crate types.
pub use compositor_background_two_draw_tile_base::{LevelMeta, RectF64, TileError};
pub use compositor_background_two_draw_tile_core::TileIndex;
pub use compositor_background_two_draw_tile_gen::{compute_max_level, generate_pyramid};
pub use compositor_background_two_draw_tile_io::{cache_dir, load_or_generate, load_tile_bytes};
