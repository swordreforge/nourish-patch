# Wallpaper: tiled giant-image background for the y5 world

Replace (or supplement) the procedural `ParallaxBackground` space shader with a
user-supplied full-scene image — 10 m × 2 m or any size — streamed through a tile
pyramid so memory stays bounded regardless of source resolution.

## Why tile-based

A 10 m × 2 m image at typical input resolution is tens of thousands of pixels wide.
A full decode + GPU upload at once uses hundreds of megabytes or more. Tiling splits
the image into a LOD pyramid of small (512×512) tiles that are:

- **Lazily loaded** — only tiles covering the current viewport are decoded and uploaded.
- **MIP-like** — each LOD is a clean 2× downscale of the previous level, so any zoom
  level samples the right amount of detail.
- **Evictable** — an LRU cache caps VRAM to ~64 MB regardless of source size.

---

## 1. Existing background pipeline (for context)

```
                 CameraMoved / CameraZoomed
                            │
                            ▼
  TwoSystem::on_camera_moved / on_camera_zoomed
  ──────────────────────────────────────────────
    writes TwoCmd::Pan(x, y) / TwoCmd::Zoom(z)

                            │
                            ▼
  TwoSystem::buffer()
  ──────────────────
    matches TwoCmd::Pan/Zoom
    → instance.pan = (x, y)
    → instance.zoom = z

  TwoSystem::draw()
  ─────────────────
    pushes Box::new(instance.clone()) into world FramePlan
    (at layer::BACKGROUND)

                            │
                            ▼
  Orchestration scene frame (scene.rs :287)
  ─────────────────────────────────────────
    drains world FramePlan
    node.downcast::<ParallaxBackground>()   ← THE key integration point
    → cloned as prepared.background_two

                            │
                            ▼
  Pushes DrawNode::Background2D(bg)
  (at layer::BACKGROUND or FLOATING_BG for panes)

                            │
                            ▼
  DrawNode::lower() → SceneElement::Background2D(e)
  → smithay composes into the final frame
```

The same pattern repeats in **picker**, **lock**, **camera**, and **graphic.capture**
scenes — each accesses `BG_TWO_MUT` or drains the world's FramePlan to extract the
background element.

---

## 2. New tile system (`draw.tile/`)

Pure data management with no smithay rendering deps.

### Tile pyramid filesystem

```
~/.config/y5/wallpaper/<sha256(source_path)>.cache/
  index.json
  L0/    000_000.png            ← single tile, entire image at coarsest LOD
  L1/    000_000.png  001_000.png
  L2/    000_000.png  001_000.png  002_000.png  003_000.png
  ...
```

`index.json`:
```json
{
  "source": "/path/to/wallpaper.png",
  "source_w": 59055,
  "source_h": 11811,
  "tile_size": 512,
  "levels": [
    { "level": 0, "w": 512,  "h": 512,  "cols": 1, "rows": 1 },
    { "level": 1, "w": 1024, "h": 512,  "cols": 2, "rows": 1 },
    { "level": 2, "w": 2048, "h": 1024, "cols": 4, "rows": 2 },
    { "level": 3, "w": 4096, "h": 2048, "cols": 8, "rows": 4 }
  ]
}
```

Each LOD is a clean 2× geometric downscale (area-averaged). The coarsest LOD (level 0)
covers the full image in one or a few tiles; the finest LOD is as close to original
resolution as possible.

### Key types

```rust
pub struct TileIndex {
    source_w: u32, source_h: u32,
    tile_size: u32,
    levels: Vec<LevelMeta>,
}
pub struct LevelMeta { level: u8, w: u32, h: u32, cols: u32, rows: u32 }

pub struct TileCache {
    index: TileIndex,
    /// GPU textures for tiles currently in VRAM. Key = (lod, col, row).
    tiles: LruCache<(u8, u32, u32), GlesTexture>,
}
```

### Public API

```rust
impl TileIndex {
    /// Load or generate the pyramid for a source image.
    /// - If cached on disk, load index.json.
    /// - If not cached, stream-decode source → generate LODs → write tiles.
    pub fn load_or_generate(path: &Path) -> Result<Self>;

    /// Choose LOD so 1 tile pixel ≈ 1 screen pixel at given zoom/screen size.
    pub fn select_lod(&self, zoom: f64, screen_w: f64, _: f64) -> u8;

    /// Enumerate tile coordinates (lod, col, row) covering a world-space rect.
    pub fn covering_tiles(&self, lod: u8, world_rect: &Rectangle<f64, Logical>)
        -> Vec<(u8, u32, u32)>;
}

impl TileCache {
    /// Ensure a tile is in VRAM. Returns false if still loading (caller uses
    /// lower-LOD placeholder).
    pub fn ensure_tile(&mut self, gles: &mut GlesRenderer, lod: u8, col: u32, row: u32) -> bool;

    /// Draw a single tile quad into the current render target.
    pub fn blit_tile(&self, gles: &mut GlesRenderer, lod: u8, col: u32, row: u32,
                     dst: Rectangle<i32, Physical>);
}
```

### Preprocessing flow

```
user sets wallpaper path
  → TileIndex::load_or_generate(path)
    → hash(path), check ~/.config/y5/wallpaper/<hash>.cache/
      → exists: read index.json, return.
      → missing: build pyramid:

          for lod from 0 to max_level:
            1. Downscale source to (level.w × level.h) — strip-based,
               never holding the full source in RAM (image crate streaming decode).
            2. Slice into tile_size × tile_size PNG files.
            3. Write each tile to L<lod>/<col>_<row>.png.

          Write index.json.

          Strip-based flow: peak memory ≈ tile_size² × 4 × 3 rows ≈ 3 MB
          + decode window × few rows ≈ 5-10 MB total.
```

---

## 3. New rendering element (`draw.wallpaper/`)

`WallpaperDrawer` is a lightweight struct that takes a `TileCache` + current camera
state and draws the visible tiles. It is NOT itself a `RenderElement` — instead, it is
called FROM the existing rendering path.

### Approach: extend `ParallaxBackground.draw()` to support wallpaper

This is the minimal-integration strategy: `ParallaxBackground` gains an optional
`wallpaper: Option<TileCache>` field. In `draw()`:

```rust
// Inside ParallaxBackground::draw():
if let Some(cache) = &self.wallpaper {
    // --- WALLPAPER PATH ---
    // Compute visible image rect from self.pan/self.zoom
    // Select LOD
    // Enumerate covering tiles
    // For each tile:
    //   if cache has it in VRAM → blit fullscreen quad
    //   else → ensure_tile() starts async load; use lower-LOD placeholder
    return Ok(());
}
// --- SHADER PATH (unchanged) ---
self.draw_pixel_program(frame, ...);
```

This works because:

1. **`ParallaxBackground` is already a `RenderElement`** — no new element type needed.
2. **`bind_pane()` already works** — the same crop/clip logic applies.
3. **All scene code untouched** — orchestration, picker, lock, camera, capture all
   continue to downcast and push `DrawNode::Background2D(bg)`.
4. **The `draw()` method has access to `GlesRenderer`** via downcast of `frame`.

For the GLES path, the tile blit uses:
```rust
fn blit_tile(gles: &mut GlesRenderer, tile: &GlesTexture, dst: Rectangle<i32, Physical>) {
    gles.bind(tile).ok();           // bind tile as read framebuffer
    gles.blit(..., dst, Linear);    // blit to current draw framebuffer
}
```

For the Vulkan path, the tile is drawn as a textured fullscreen quad using a simple
sampler shader.

### State flow

```
Two {
    instance: Option<ParallaxBackground>,   // same as today
    wallpaper: Option<WallpaperState>,       // new
    background_shader: ...,                  // unchanged, ignored when wallpaper set
    ...
}
```

- `wallpaper = None` → `ParallaxBackground` runs the shader, exactly as today.
- `wallpaper = Some(...)` → `ParallaxBackground` gets `.wallpaper = Some(cache)` set,
  and `draw()` takes the wallpaper path.

The `TwoSystem` decides which to use:

```rust
// In TwoSystem::update():
if two.wallpaper.is_some() {
    // Skin the ParallaxBackground with the tile cache.
    // The instance still exists but draws tiles instead of shader.
    if let Some(inst) = &mut two.instance {
        inst.wallpaper = Some(two.wallpaper_cache.clone());
    }
} else {
    // Shader path: ensure instance is created, clear wallpaper field.
    if let Some(inst) = &mut two.instance {
        inst.wallpaper = None;
    }
}
```

---

## 4. Full integration chain (end to end)

```
User selects wallpaper image
        │
        ▼
Settings interface writes world.background.json
{ "wallpaper": "/path/to/10m×2m.png" }
        │
        ▼
Persist system rehydrates Two at next world build
→ Two.wallpaper = Some(WallpaperState { path, ... })
        │
        ▼
TwoSystem::update()
├─ wallpaper.is_some()
│   ├─ TileIndex::load_or_generate(path)  ← generates/loads tile pyramid
│   ├─ creates TileCache from index
│   └─ sets instance.wallpaper = Some(cache)
│
└─ wallpaper.is_none()
    └─ clears instance.wallpaper, creates ParallaxBackground (shader) as today
        │
        ▼
TwoSystem handles CAMERA_MOVED / CAMERA_ZOOMED
→ instance.pan, instance.zoom updated (same as today)
        │
        ▼
TwoSystem::draw()
→ pushes Box::new(instance.clone()) into world FramePlan
→ (instance is still ParallaxBackground, now with optional wallpaper cache)
        │
        ▼
Orchestration scene frame (scene.rs :287)
→ node.downcast::<ParallaxBackground>() ← WORKS, same type
→ cloned as prepared.background_two
        │
        ▼
Pushes DrawNode::Background2D(bg) / Background2DCropped
→ unchanged code path
        │
        ▼
DrawNode::lower() → SceneElement::Background2D(e)
        │
        ▼
smithay render → calls ParallaxBackground::draw()
├─ instance.wallpaper.is_some()
│   → compute visible tiles from pan/zoom
│   → for each tile in LRU: blit textured quad
│   → tiles not yet loaded: use lower-LOD placeholder
│
└─ wallpaper.is_none()
    → run pixel program (shader) as today
```

### Modified files (complete list)

| File | Change |
|---|---|
| **New:** `background.two/two.draw/draw.tile/` | Tile pyramid: generation, IO, LRU cache |
| **New:** `background.two/two.draw/draw.wallpaper/` | Tile rendering in GLES/Vulkan (called from ParallaxBackground::draw) |
| `two.state/state.base/state.rs` | `Two` gets `wallpaper: Option<WallpaperState>` |
| `two.storage/storage.base/base.rs` | `BackgroundPersisted` / `BackgroundPersist` adds `wallpaper_path` |
| `two.system/system.base/base.rs` | `update()` sets `instance.wallpaper` from `Two.wallpaper` |
| `two.draw/draw.parallax/parallax.rs` | `draw()`: `if wallpaper.is_some() { render_tiles() } else { shader() }`; struct gains `wallpaper: Option<TileCache>` |

**NO changes** to:
- `compositor.orchestration` (DrawNode, SceneElement, scene.frame)
- Picker / lock / camera / capture scenes
- ParallaxBackground's `RenderElement` impl signature

---

## 5. LOD selection & coordinate math

### World-space image rectangle

The image lives at the world origin. The image occupies `IMAGE_W` meters in world x
and `IMAGE_H` meters in world y:

```
world_w = IMAGE_W       (e.g. 10.0)
world_h = IMAGE_H       (e.g. 2.0)
```

### Viewport → image mapping

```
viewport_world_w = screen_w / zoom
viewport_world_h = screen_h / zoom
viewport_left    = pan_x - viewport_world_w / 2
viewport_top     = pan_y - viewport_world_h / 2
```

Intersect this rect with `[0, world_w] × [0, world_h]` to get the visible image
region. Clamp viewport rect to image bounds.

### LOD selection

```
visible_ratio = min(1.0, visible_w / world_w)  ← fraction of full image width visible
visible_pixels = screen_w                      ← screen pixels available
source_pixels_needed = visible_pixels / visible_ratio

lod = floor(log2(source_w / source_pixels_needed))
lod = clamp(lod, 0, max_level)
```

### Tile enumeration

```
lod_w = index.levels[lod].w
lod_h = index.levels[lod].h
tile_size = 512

image_x_start = max(0, viewport_left / world_w * lod_w)
image_y_start = max(0, viewport_top / world_h * lod_h)
image_x_end   = min(lod_w, (viewport_left + viewport_world_w) / world_w * lod_w)
image_y_end   = min(lod_h, (viewport_top + viewport_world_h) / world_h * lod_h)

col_start = floor(image_x_start / tile_size)
col_end   = floor(image_x_end / tile_size)
row_start = floor(image_y_start / tile_size)
row_end   = floor(image_y_end / tile_size)

covered = [(lod, c, r) for c in col_start..=col_end for r in row_start..=row_end]
```

### Tile → screen coordinate for blit

```
screen_x = (col * tile_size / lod_w * world_w - viewport_left) / viewport_world_w * screen_w
screen_y = similar
screen_w_tile = (tile_size / lod_w * world_w) / viewport_world_w * screen_w
```

---

## 6. Memory budget

| Phase | What | Peak |
|---|---|---|
| Preprocessing | Strip decode buffer + output tiles | ~10 MB |
| Steady-state | LRU GPU tiles (64 × 1 MB) | 64 MB VRAM |
| I/O burst | One tile being decoded (CPU) | ~1 MB |
| Total VRAM | LRU + compositor framebuffers | ~100 MB |

No dynamic allocation proportional to source resolution.

---

## 7. Performance properties

| Scenario | Tiles/frame | Draw cost |
|---|---|---|
| zoom=1.0, full image visible, 4K screen | 8 × 2 = 16 tiles | ~0.02 ms |
| zoomed in, 1920×1080 region visible | 4 × 2 = 8 tiles | ~0.01 ms |
| zoomed to detail, 512×512 region | 1 tile | ~0.001 ms |
| Fast panning (cache misses) | +streaming from disk | <1 ms stall per miss |

- **Cache hit**: pure GPU blit, negligible overhead.
- **Cache miss**: disk I/O + PNG decode (the first cut does this inline; an async
  threadpool is future work if stutter is observed).
- **Placeholder fallback**: uses next-lower LOD via already-cached tile, upscaled
  linear. No visual hole.

---

## 8. Implementation order (MVP)

| Step | Crate | What |
|---|---|---|
| 1 | `two.draw/draw.tile/` | `TileIndex`, `TileCache`, `load_or_generate()` |
| 2 | `two.draw/draw.wallpaper/` | `render_tiles()` function: visible rect computation, LOD select, tile blit |
| 3 | `two.state` + `two.storage` | `WallpaperState` struct, persist field, settings plumbing |
| 4 | `two.draw/draw.parallax` | `ParallaxBackground.wallpaper` field, `draw()` dispatches to tile path |
| 5 | `two.system` | `update()` wires wallpaper → tile cache → parallax instance |
| 6 | Test | Build, run, verify tile generation and rendering |

---

## 9. Future work (not in MVP)

- **Async tile loading threadpool** — avoid main-thread decode stalls.
- **Panorama / 360°** — cylindrical projection, `pan_x` wraps as azimuth.
- **Multi-monitor spanning** — shared wallpaper across all outputs.
- **Video wallpaper** — per-frame LOD tree from a decoder.
- **Parallax factor** — wallpaper drifts slower/faster than foreground.
- **Interactive hotspots** — tap a region of the wallpaper to navigate.
