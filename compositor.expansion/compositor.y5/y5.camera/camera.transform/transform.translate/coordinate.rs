// //! Typed coordinate spaces for y5, with implicit conversions.
// //!
// //! ## What this module is for
// //!
// //! y5 deals with values that exist in three different spaces and two
// //! numeric representations. Without typed spaces it's very easy to
// //! pass a screen-space value somewhere that wants world, or a
// //! physical pixel count somewhere that wants logical. Compiler can't
// //! help, and the bugs manifest as "cursor stops at half the screen"
// //! or "window renders 200px off."
// //!
// //! `Coordinate<S, T>` makes the space and numeric type part of the
// //! type signature, with `.into()` converting between them implicitly.
// //!
// //! ## Spaces
// //!
// //! - **`Physical`**: framebuffer / DRM scanout pixels. Top-left =
// //!   (0, 0); bottom-right = the panel's mode size (e.g. 5120×1440).
// //!   The **canonical internal representation** — every Coordinate
// //!   stores its value here, in `f64`.
// //!
// //! - **`Screen`**: logical pixels. Top-left = (0, 0); bottom-right =
// //!   `physical_size / scale` (e.g. ~2926×823 with fractional=1.75 on
// //!   a 5120×1440 panel). What Wayland clients see for pointer events
// //!   and surface positions. Smithay's `Logical`.
// //!
// //! - **`World`**: y5's panable, zoomable canvas. Where windows live.
// //!   Conversion through Screen via the camera transform.
// //!
// //! ## What a `Coordinate` is
// //!
// //! A `Coordinate` carries **both a position and a size**. The
// //! position is the top-left corner; the size is the width and
// //! height. A "point" is a Coordinate with zero size; a "size"
// //! (extent / dimensions) is a Coordinate with zero position. The
// //! generic case is a rectangle.
// //!
// //! Position and size convert differently between spaces:
// //!
// //! - Position is anchored. Screen→World subtracts the camera offset
// //!   and scales by zoom. Position cares about origin.
// //! - Size is a delta. Screen→World only scales by zoom. Size doesn't
// //!   care about origin.
// //!
// //! Both are stored canonically as Physical f64 and converted on
// //! demand to whatever the destination type asks for.
// //!
// //! ## Usage
// //!
// //! ```ignore
// //! // Build the per-frame context snapshot once:
// //! let ctx = Context::new(
// //!     camera_pos = cam.position_tuple(),
// //!     camera_zoom = cam.zoom(),
// //!     screen_size_physical = (mode.size.w as f64, mode.size.h as f64),
// //!     scale = output.current_scale().fractional_scale(),
// //! );
// //!
// //! // A cursor (point):
// //! let cursor = Coordinate::<Screen, f64>::point(120.0, 80.0, ctx);
// //!
// //! // A window (full rect):
// //! let window = Coordinate::<World, f64>::rect(100.0, 50.0, 800.0, 600.0, ctx);
// //!
// //! // A bare size (no position):
// //! let buffer_size = Coordinate::<Physical, i32>::size(1920, 1080, ctx);
// //!
// //! // Function declares what it wants:
// //! fn render_thing(p: Coordinate<Screen, f64>) { /* ... */ }
// //! fn upload_to_gpu(p: Coordinate<Physical, i32>) { /* ... */ }
// //!
// //! // Implicit conversion via Into:
// //! render_thing(window.into());      // Coord<World, f64> → Coord<Screen, f64>
// //! upload_to_gpu(cursor.into());     // Coord<Screen, f64> → Coord<Physical, i32>
// //! ```
// //!
// //! ## Context as snapshot
// //!
// //! `Context` is `Copy` and frozen at Coordinate construction. If the
// //! camera moves mid-frame, it's the caller's responsibility to
// //! construct fresh Coordinates with the new Context. This is a
// //! deliberate design choice: lazy reads of mutable state make
// //! coordinate values non-reproducible and hard to debug.
//
// use std::marker::PhantomData;
// use std::ops::{Add, Sub};
//
// use smithay::utils as su;
//
// // ─── Space markers ──────────────────────────────────────────────────
//
// #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
// pub struct Screen;
// #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
// pub struct Physical;
// #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
// pub struct World;
//
// pub trait Space: sealed::Sealed + Copy + 'static {}
// impl Space for Screen {}
// impl Space for Physical {}
// impl Space for World {}
//
// // ─── Numeric markers ───────────────────────────────────────────────
//
// pub trait Numeric: sealed::Sealed + Copy + 'static {}
// impl Numeric for f64 {}
// impl Numeric for i32 {}
//
// mod sealed {
//     pub trait Sealed {}
//     impl Sealed for super::Screen {}
//     impl Sealed for super::Physical {}
//     impl Sealed for super::World {}
//     impl Sealed for f64 {}
//     impl Sealed for i32 {}
// }
//
// // ─── Context: snapshot of the conversion data ──────────────────────
//
// /// Frozen snapshot of the per-frame conversion data.
// ///
// /// Construct this once per frame from the live state, then pass it
// /// into every `Coordinate` you build. Cheap to copy; lives inside
// /// every Coordinate instance.
// ///
// /// **`screen_size_physical` is the canonical screen size** — the
// /// panel's real pixel mode (e.g. 5120×1440). Screen-logical is
// /// derived as `physical / scale`.
// #[derive(Debug, Clone, Copy)]
// pub struct Context {
//     /// Camera position in **world** space.
//     pub camera_pos: (f64, f64),
//     /// Camera zoom (1.0 = no zoom; > 1 = zoomed in).
//     pub camera_zoom: f64,
//     /// Screen size in **physical** units (panel mode size).
//     pub screen_size_physical: (f64, f64),
//     /// Fractional scale factor relating Screen to Physical.
//     /// Physical = Screen × scale.
//     pub scale: f64,
// }
//
// impl Context {
//     pub fn new(
//         camera_pos: (f64, f64),
//         camera_zoom: f64,
//         screen_size_physical: (f64, f64),
//         scale: f64,
//     ) -> Self {
//         Self {
//             camera_pos,
//             camera_zoom,
//             screen_size_physical,
//             scale,
//         }
//     }
//
//     /// Screen size in **logical** units, derived from physical / scale.
//     #[inline]
//     fn screen_size_logical(&self) -> (f64, f64) {
//         (
//             self.screen_size_physical.0 / self.scale,
//             self.screen_size_physical.1 / self.scale,
//         )
//     }
// }
//
// // ─── Coordinate: position + size, typed by space and numeric ───────
//
// /// A coordinate in space `S` with numeric type `T`. Carries both a
// /// **position** (top-left corner) and a **size** (width and height).
// ///
// /// Internally always stored as Physical f64, plus a `Context`
// /// snapshot. The type parameters `S, T` only control how values are
// /// read back via `.x()`, `.y()`, `.w()`, `.h()`, and the `From`
// /// conversions.
// #[derive(Debug, Clone, Copy)]
// pub struct Coordinate<S: Space, T: Numeric> {
//     /// Canonical position (top-left) in Physical f64.
//     pos_physical: (f64, f64),
//     /// Canonical size (w, h) in Physical f64. Zero for "this is just
//     /// a point."
//     size_physical: (f64, f64),
//     /// Context snapshot (camera, screen, scale at construction).
//     ctx: Context,
//     _space: PhantomData<S>,
//     _numeric: PhantomData<T>,
// }
//
// impl Coordinate<World, f64> {
//     /// Construct a World Coordinate from a smithay `Rectangle<i32, Logical>`,
//     /// treating the smithay values as raw world coordinates (no camera
//     /// transform). Use this when smithay's Space stores world positions
//     /// (the common case in y5).
//     pub fn from_smithay_world_rect(r: su::Rectangle<i32, su::Logical>, ctx: Context) -> Self {
//         Self::rect(r.loc.x as f64, r.loc.y as f64, r.size.w as f64, r.size.h as f64, ctx)
//     }
// }
//
// impl Coordinate<World, i32> {
//     /// Extract the raw world coordinates as a smithay `Point<i32, Logical>`,
//     /// suitable for storing in smithay's Space when y5's convention is
//     /// that smithay's Space holds world coords.
//     pub fn into_smithay_world_logical_point(self) -> su::Point<i32, su::Logical> {
//         // We want the raw World numbers, NOT the Screen projection.
//         // Extract via the World accessor.
//         let (x, y) = self.pos_tuple();
//         su::Point::from((x, y))
//     }
// }
//
// // ─── Construction: point / size / rect ─────────────────────────────
// //
// // Three constructors. Each one takes values in the type-parameter's
// // (S, T) and immediately normalises to canonical Physical f64.
// //
// // We expose this for the six (Space, Numeric) combinations via a
// // macro so the constructor names are uniform.
//
// macro_rules! impl_constructors {
//     ($space:ty, $num:ty) => {
//         impl Coordinate<$space, $num> {
//             /// A point: position only, zero size. The point is at
//             /// `(x, y)` in this Coordinate's space.
//             pub fn point(x: $num, y: $num, ctx: Context) -> Self {
//                 let pos = pos_to_physical::<$space>((x as f64, y as f64), ctx);
//                 Self {
//                     pos_physical: pos,
//                     size_physical: (0.0, 0.0),
//                     ctx,
//                     _space: PhantomData,
//                     _numeric: PhantomData,
//                 }
//             }
//
//             /// A bare size: position at origin (0, 0), with the given
//             /// width and height in this Coordinate's space.
//             pub fn size(w: $num, h: $num, ctx: Context) -> Self {
//                 let size = size_to_physical::<$space>((w as f64, h as f64), ctx);
//                 Self {
//                     pos_physical: (0.0, 0.0),
//                     size_physical: size,
//                     ctx,
//                     _space: PhantomData,
//                     _numeric: PhantomData,
//                 }
//             }
//
//             /// A rectangle: position and size, both in this
//             /// Coordinate's space.
//             pub fn rect(x: $num, y: $num, w: $num, h: $num, ctx: Context) -> Self {
//                 let pos = pos_to_physical::<$space>((x as f64, y as f64), ctx);
//                 let size = size_to_physical::<$space>((w as f64, h as f64), ctx);
//                 Self {
//                     pos_physical: pos,
//                     size_physical: size,
//                     ctx,
//                     _space: PhantomData,
//                     _numeric: PhantomData,
//                 }
//             }
//         }
//     };
// }
//
// impl_constructors!(Screen, f64);
// impl_constructors!(Screen, i32);
// impl_constructors!(Physical, f64);
// impl_constructors!(Physical, i32);
// impl_constructors!(World, f64);
// impl_constructors!(World, i32);
//
// // ─── Accessors ────────────────────────────────────────────────────
//
// impl<S: Space, T: Numeric> Coordinate<S, T>
// where
//     S: SpaceConvert,
//     T: ExtractAs,
// {
//     /// X coordinate of the top-left corner, in this `(S, T)`.
//     pub fn x(&self) -> T {
//         T::from_f64(pos_from_physical::<S>(self.pos_physical, self.ctx).0)
//     }
//
//     /// Y coordinate of the top-left corner, in this `(S, T)`.
//     pub fn y(&self) -> T {
//         T::from_f64(pos_from_physical::<S>(self.pos_physical, self.ctx).1)
//     }
//
//     /// Width.
//     pub fn w(&self) -> T {
//         T::from_f64(size_from_physical::<S>(self.size_physical, self.ctx).0)
//     }
//
//     /// Height.
//     pub fn h(&self) -> T {
//         T::from_f64(size_from_physical::<S>(self.size_physical, self.ctx).1)
//     }
//
//     /// Right edge: `x + w`.
//     pub fn right(&self) -> T {
//         let pos = pos_from_physical::<S>(self.pos_physical, self.ctx);
//         let size = size_from_physical::<S>(self.size_physical, self.ctx);
//         T::from_f64(pos.0 + size.0)
//     }
//
//     /// Bottom edge: `y + h`.
//     pub fn bottom(&self) -> T {
//         let pos = pos_from_physical::<S>(self.pos_physical, self.ctx);
//         let size = size_from_physical::<S>(self.size_physical, self.ctx);
//         T::from_f64(pos.1 + size.1)
//     }
//
//     /// `(x, y)` as a tuple.
//     pub fn pos_tuple(&self) -> (T, T) {
//         let pos = pos_from_physical::<S>(self.pos_physical, self.ctx);
//         (T::from_f64(pos.0), T::from_f64(pos.1))
//     }
//
//     /// `(w, h)` as a tuple.
//     pub fn size_tuple(&self) -> (T, T) {
//         let size = size_from_physical::<S>(self.size_physical, self.ctx);
//         (T::from_f64(size.0), T::from_f64(size.1))
//     }
// }
//
// impl<S: Space, T: Numeric> Coordinate<S, T> {
//     /// The frozen Context this Coordinate was constructed with.
//     pub fn context(&self) -> Context {
//         self.ctx
//     }
// }
//
// // ─── Position math: depends on the space ───────────────────────────
// //
// // Positions are anchored: Screen→World subtracts the camera offset
// // and divides by zoom. Position cares about origin.
// //
// // All paths funnel through canonical Physical f64.
//
// fn pos_to_physical<S: SpaceConvert>(in_space: (f64, f64), ctx: Context) -> (f64, f64) {
//     S::pos_to_physical(in_space, ctx)
// }
//
// fn pos_from_physical<S: SpaceConvert>(physical: (f64, f64), ctx: Context) -> (f64, f64) {
//     S::pos_from_physical(physical, ctx)
// }
//
// // ─── Size math: depends on the space, but ignores camera offset ────
// //
// // Sizes are deltas. World→Screen only scales by camera zoom (no
// // translation). Screen→Physical multiplies by fractional scale.
//
// fn size_to_physical<S: SpaceConvert>(in_space: (f64, f64), ctx: Context) -> (f64, f64) {
//     S::size_to_physical(in_space, ctx)
// }
//
// fn size_from_physical<S: SpaceConvert>(physical: (f64, f64), ctx: Context) -> (f64, f64) {
//     S::size_from_physical(physical, ctx)
// }
//
// // ─── Per-space conversion trait ────────────────────────────────────
//
// pub trait SpaceConvert {
//     fn pos_to_physical(p: (f64, f64), ctx: Context) -> (f64, f64);
//     fn pos_from_physical(p: (f64, f64), ctx: Context) -> (f64, f64);
//     fn size_to_physical(s: (f64, f64), ctx: Context) -> (f64, f64);
//     fn size_from_physical(s: (f64, f64), ctx: Context) -> (f64, f64);
// }
//
// impl SpaceConvert for Physical {
//     #[inline]
//     fn pos_to_physical(p: (f64, f64), _: Context) -> (f64, f64) {
//         p
//     }
//     #[inline]
//     fn pos_from_physical(p: (f64, f64), _: Context) -> (f64, f64) {
//         p
//     }
//     #[inline]
//     fn size_to_physical(s: (f64, f64), _: Context) -> (f64, f64) {
//         s
//     }
//     #[inline]
//     fn size_from_physical(s: (f64, f64), _: Context) -> (f64, f64) {
//         s
//     }
// }
//
// impl SpaceConvert for Screen {
//     #[inline]
//     fn pos_to_physical(p: (f64, f64), ctx: Context) -> (f64, f64) {
//         (p.0 * ctx.scale, p.1 * ctx.scale)
//     }
//     #[inline]
//     fn pos_from_physical(p: (f64, f64), ctx: Context) -> (f64, f64) {
//         (p.0 / ctx.scale, p.1 / ctx.scale)
//     }
//     #[inline]
//     fn size_to_physical(s: (f64, f64), ctx: Context) -> (f64, f64) {
//         // Size in Screen → Physical: multiply by scale. Same as
//         // position because scale is uniform (no origin distinction).
//         (s.0 * ctx.scale, s.1 * ctx.scale)
//     }
//     #[inline]
//     fn size_from_physical(s: (f64, f64), ctx: Context) -> (f64, f64) {
//         (s.0 / ctx.scale, s.1 / ctx.scale)
//     }
// }
//
// impl SpaceConvert for World {
//     fn pos_to_physical(p: (f64, f64), ctx: Context) -> (f64, f64) {
//         // World → Screen: anchored at camera, zoomed, centred.
//         let (half_w, half_h) = {
//             let logical = ctx.screen_size_logical();
//             (logical.0 / 2.0, logical.1 / 2.0)
//         };
//         let screen_x = (p.0 - ctx.camera_pos.0) * ctx.camera_zoom + half_w;
//         let screen_y = (p.1 - ctx.camera_pos.1) * ctx.camera_zoom + half_h;
//         // Screen → Physical:
//         (screen_x * ctx.scale, screen_y * ctx.scale)
//     }
//
//     fn pos_from_physical(p: (f64, f64), ctx: Context) -> (f64, f64) {
//         // Physical → Screen:
//         let screen_x = p.0 / ctx.scale;
//         let screen_y = p.1 / ctx.scale;
//         let (half_w, half_h) = {
//             let logical = ctx.screen_size_logical();
//             (logical.0 / 2.0, logical.1 / 2.0)
//         };
//         // Screen → World: invert centre + zoom + camera offset.
//         (
//             (screen_x - half_w) / ctx.camera_zoom + ctx.camera_pos.0,
//             (screen_y - half_h) / ctx.camera_zoom + ctx.camera_pos.1,
//         )
//     }
//
//     fn size_to_physical(s: (f64, f64), ctx: Context) -> (f64, f64) {
//         // World → Screen: just zoom (no centre, no camera offset
//         // because size is a delta, not a position).
//         let screen = (s.0 * ctx.camera_zoom, s.1 * ctx.camera_zoom);
//         // Screen → Physical:
//         (screen.0 * ctx.scale, screen.1 * ctx.scale)
//     }
//
//     fn size_from_physical(s: (f64, f64), ctx: Context) -> (f64, f64) {
//         // Physical → Screen:
//         let screen = (s.0 / ctx.scale, s.1 / ctx.scale);
//         // Screen → World: divide by zoom.
//         (screen.0 / ctx.camera_zoom, screen.1 / ctx.camera_zoom)
//     }
// }
//
// // ─── Numeric extraction ────────────────────────────────────────────
//
// pub trait ExtractAs: Numeric {
//     fn from_f64(v: f64) -> Self;
// }
//
// impl ExtractAs for f64 {
//     #[inline]
//     fn from_f64(v: f64) -> Self {
//         v
//     }
// }
//
// impl ExtractAs for i32 {
//     #[inline]
//     fn from_f64(v: f64) -> Self {
//         v.round() as i32
//     }
// }
//
// // ─── Implicit conversions: From / Into ─────────────────────────────
// //
// // Every non-identity (S₁, T₁) → (S₂, T₂) pair gets a From impl. All
// // 30 of them are mechanical — the canonical Physical f64 storage is
// // space-and-precision-independent, so changing the markers is the
// // entire conversion at the type-system level. The numeric conversion
// // happens lazily on accessor read.
// //
// // We use a macro to generate the impls.
//
// macro_rules! impl_from {
//     ($from_s:ty, $from_t:ty => $to_s:ty, $to_t:ty) => {
//         impl From<Coordinate<$from_s, $from_t>> for Coordinate<$to_s, $to_t> {
//             fn from(c: Coordinate<$from_s, $from_t>) -> Self {
//                 Coordinate {
//                     pos_physical: c.pos_physical,
//                     size_physical: c.size_physical,
//                     ctx: c.ctx,
//                     _space: PhantomData,
//                     _numeric: PhantomData,
//                 }
//             }
//         }
//     };
// }
//
// // 30 impls covering every non-identity pair.
// // Identity pairs (e.g. Screen f64 → Screen f64) come from std's
// // blanket `impl<T> From<T> for T`.
//
// impl_from!(Screen, f64 => Screen, i32);
// impl_from!(Screen, f64 => Physical, f64);
// impl_from!(Screen, f64 => Physical, i32);
// impl_from!(Screen, f64 => World, f64);
// impl_from!(Screen, f64 => World, i32);
//
// impl_from!(Screen, i32 => Screen, f64);
// impl_from!(Screen, i32 => Physical, f64);
// impl_from!(Screen, i32 => Physical, i32);
// impl_from!(Screen, i32 => World, f64);
// impl_from!(Screen, i32 => World, i32);
//
// impl_from!(Physical, f64 => Screen, f64);
// impl_from!(Physical, f64 => Screen, i32);
// impl_from!(Physical, f64 => Physical, i32);
// impl_from!(Physical, f64 => World, f64);
// impl_from!(Physical, f64 => World, i32);
//
// impl_from!(Physical, i32 => Screen, f64);
// impl_from!(Physical, i32 => Screen, i32);
// impl_from!(Physical, i32 => Physical, f64);
// impl_from!(Physical, i32 => World, f64);
// impl_from!(Physical, i32 => World, i32);
//
// impl_from!(World, f64 => Screen, f64);
// impl_from!(World, f64 => Screen, i32);
// impl_from!(World, f64 => Physical, f64);
// impl_from!(World, f64 => Physical, i32);
// impl_from!(World, f64 => World, i32);
//
// impl_from!(World, i32 => Screen, f64);
// impl_from!(World, i32 => Screen, i32);
// impl_from!(World, i32 => Physical, f64);
// impl_from!(World, i32 => Physical, i32);
// impl_from!(World, i32 => World, f64);
//
// // ─── Arithmetic (same (S, T) only) ─────────────────────────────────
// //
// // Adding two same-space Coordinates: positions add, sizes add. Useful
// // for "displace this rect by that offset, growing by that size."
// //
// // Math is exact: it operates on canonical Physical f64.
//
// impl<S: Space, T: Numeric> Add for Coordinate<S, T> {
//     type Output = Self;
//     fn add(self, rhs: Self) -> Self {
//         Coordinate {
//             pos_physical: (
//                 self.pos_physical.0 + rhs.pos_physical.0,
//                 self.pos_physical.1 + rhs.pos_physical.1,
//             ),
//             size_physical: (
//                 self.size_physical.0 + rhs.size_physical.0,
//                 self.size_physical.1 + rhs.size_physical.1,
//             ),
//             ctx: self.ctx,
//             _space: PhantomData,
//             _numeric: PhantomData,
//         }
//     }
// }
//
// impl<S: Space, T: Numeric> Sub for Coordinate<S, T> {
//     type Output = Self;
//     fn sub(self, rhs: Self) -> Self {
//         Coordinate {
//             pos_physical: (
//                 self.pos_physical.0 - rhs.pos_physical.0,
//                 self.pos_physical.1 - rhs.pos_physical.1,
//             ),
//             size_physical: (
//                 self.size_physical.0 - rhs.size_physical.0,
//                 self.size_physical.1 - rhs.size_physical.1,
//             ),
//             ctx: self.ctx,
//             _space: PhantomData,
//             _numeric: PhantomData,
//         }
//     }
// }
//
// // ─── Bridges to smithay's Point<T, Logical/Physical> ───────────────
// //
// // Smithay has Logical and Physical markers, no World. Bridges are
// // between our markers and theirs:
// //
// //   smithay Logical  ⇄ our Screen
// //   smithay Physical ⇄ our Physical
// //
// // For full Rectangles see below.
//
// impl Coordinate<Screen, f64> {
//     pub fn into_smithay_logical_point(self) -> su::Point<f64, su::Logical> {
//         su::Point::from(self.pos_tuple())
//     }
//     pub fn from_smithay_logical_point(p: su::Point<f64, su::Logical>, ctx: Context) -> Self {
//         Self::point(p.x, p.y, ctx)
//     }
//     pub fn into_smithay_logical_size(self) -> su::Size<f64, su::Logical> {
//         let (w, h) = self.size_tuple();
//         su::Size::from((w, h))
//     }
//     pub fn into_smithay_logical_rect(self) -> su::Rectangle<f64, su::Logical> {
//         su::Rectangle::new(self.into_smithay_logical_point(), self.into_smithay_logical_size())
//     }
// }
//
// impl Coordinate<Screen, i32> {
//     pub fn into_smithay_logical_point(self) -> su::Point<i32, su::Logical> {
//         su::Point::from(self.pos_tuple())
//     }
//     pub fn from_smithay_logical_point(p: su::Point<i32, su::Logical>, ctx: Context) -> Self {
//         Self::point(p.x, p.y, ctx)
//     }
//     pub fn into_smithay_logical_size(self) -> su::Size<i32, su::Logical> {
//         let (w, h) = self.size_tuple();
//         su::Size::from((w, h))
//     }
//     pub fn into_smithay_logical_rect(self) -> su::Rectangle<i32, su::Logical> {
//         su::Rectangle::new(self.into_smithay_logical_point(), self.into_smithay_logical_size())
//     }
//     pub fn from_smithay_logical_rect(r: su::Rectangle<i32, su::Logical>, ctx: Context) -> Self {
//         Self::rect(r.loc.x, r.loc.y, r.size.w, r.size.h, ctx)
//     }
// }
//
// impl Coordinate<Physical, f64> {
//     pub fn into_smithay_physical_point(self) -> su::Point<f64, su::Physical> {
//         su::Point::from(self.pos_tuple())
//     }
//     pub fn from_smithay_physical_point(p: su::Point<f64, su::Physical>, ctx: Context) -> Self {
//         Self::point(p.x, p.y, ctx)
//     }
//     pub fn into_smithay_physical_size(self) -> su::Size<f64, su::Physical> {
//         let (w, h) = self.size_tuple();
//         su::Size::from((w, h))
//     }
//     pub fn into_smithay_physical_rect(self) -> su::Rectangle<f64, su::Physical> {
//         su::Rectangle::new(self.into_smithay_physical_point(), self.into_smithay_physical_size())
//     }
// }
//
// impl Coordinate<Physical, i32> {
//     pub fn into_smithay_physical_point(self) -> su::Point<i32, su::Physical> {
//         su::Point::from(self.pos_tuple())
//     }
//     pub fn from_smithay_physical_point(p: su::Point<i32, su::Physical>, ctx: Context) -> Self {
//         Self::point(p.x, p.y, ctx)
//     }
//     pub fn into_smithay_physical_size(self) -> su::Size<i32, su::Physical> {
//         let (w, h) = self.size_tuple();
//         su::Size::from((w, h))
//     }
//     pub fn into_smithay_physical_rect(self) -> su::Rectangle<i32, su::Physical> {
//         su::Rectangle::new(self.into_smithay_physical_point(), self.into_smithay_physical_size())
//     }
//     pub fn from_smithay_physical_rect(r: su::Rectangle<i32, su::Physical>, ctx: Context) -> Self {
//         Self::rect(r.loc.x, r.loc.y, r.size.w, r.size.h, ctx)
//     }
// }
//
// #[cfg(test)]
// mod tests {
//     use super::*;
//
//     fn ctx() -> Context {
//         Context::new(
//             (100.0, -50.0), // camera in world
//             2.0,            // zoom
//             (5120.0, 1440.0), // physical screen
//             1.75,           // fractional scale
//         )
//     }
//
//     #[test]
//     fn point_screen_to_physical() {
//         let p = Coordinate::<Screen, f64>::point(200.0, 100.0, ctx());
//         let phys: Coordinate<Physical, f64> = p.into();
//         // 200 * 1.75 = 350
//         assert!((phys.x() - 350.0).abs() < 1e-9);
//         assert!((phys.y() - 175.0).abs() < 1e-9);
//     }
//
//     #[test]
//     fn rect_screen_to_physical_scales_both() {
//         let r = Coordinate::<Screen, f64>::rect(100.0, 200.0, 400.0, 300.0, ctx());
//         let phys: Coordinate<Physical, f64> = r.into();
//         assert!((phys.x() - 175.0).abs() < 1e-9);
//         assert!((phys.y() - 350.0).abs() < 1e-9);
//         assert!((phys.w() - 700.0).abs() < 1e-9);
//         assert!((phys.h() - 525.0).abs() < 1e-9);
//     }
//
//     #[test]
//     fn size_world_to_screen_only_zooms() {
//         // World size of 100x50, with camera_zoom=2.0, should become
//         // 200x100 in screen (no translation applied to size).
//         let s = Coordinate::<World, f64>::size(100.0, 50.0, ctx());
//         let screen: Coordinate<Screen, f64> = s.into();
//         // Note: position is at world (0, 0), which is NOT at screen
//         // (0, 0) — it's at screen-centre minus camera offset.
//         // But we only care about the size here.
//         assert!((screen.w() - 200.0).abs() < 1e-9);
//         assert!((screen.h() - 100.0).abs() < 1e-9);
//     }
//
//     #[test]
//     fn size_does_not_use_camera_position() {
//         // Same world size in two different camera contexts should
//         // produce the same screen size.
//         let mut c1 = ctx();
//         c1.camera_pos = (1000.0, 2000.0);
//         let mut c2 = ctx();
//         c2.camera_pos = (-500.0, -1000.0);
//
//         let s1 = Coordinate::<World, f64>::size(100.0, 50.0, c1);
//         let s2 = Coordinate::<World, f64>::size(100.0, 50.0, c2);
//
//         let scr1: Coordinate<Screen, f64> = s1.into();
//         let scr2: Coordinate<Screen, f64> = s2.into();
//
//         assert!((scr1.w() - scr2.w()).abs() < 1e-9);
//         assert!((scr1.h() - scr2.h()).abs() < 1e-9);
//     }
//
//     #[test]
//     fn position_uses_camera_and_centre() {
//         // Camera at world (0,0), zoom=1, screen 1000x500 logical.
//         // World origin should map to screen centre (500, 250).
//         let c = Context::new((0.0, 0.0), 1.0, (1000.0, 500.0), 1.0);
//         let p = Coordinate::<World, f64>::point(0.0, 0.0, c);
//         let s: Coordinate<Screen, f64> = p.into();
//         assert!((s.x() - 500.0).abs() < 1e-9);
//         assert!((s.y() - 250.0).abs() < 1e-9);
//     }
//
//     #[test]
//     fn rect_world_to_screen_full() {
//         // Camera at world (0,0), zoom=1, screen 1000x500 logical.
//         // A world rect at (0, 0) sized 100x50 should map to a screen
//         // rect at (500, 250) sized 100x50.
//         let c = Context::new((0.0, 0.0), 1.0, (1000.0, 500.0), 1.0);
//         let r = Coordinate::<World, f64>::rect(0.0, 0.0, 100.0, 50.0, c);
//         let s: Coordinate<Screen, f64> = r.into();
//         assert!((s.x() - 500.0).abs() < 1e-9);
//         assert!((s.y() - 250.0).abs() < 1e-9);
//         assert!((s.w() - 100.0).abs() < 1e-9);
//         assert!((s.h() - 50.0).abs() < 1e-9);
//     }
//
//     #[test]
//     fn cross_type_rounds() {
//         let p = Coordinate::<Screen, f64>::rect(199.4, 100.7, 50.3, 50.6, ctx());
//         let i: Coordinate<Screen, i32> = p.into();
//         assert_eq!(i.x(), 199);
//         assert_eq!(i.y(), 101);
//         assert_eq!(i.w(), 50);
//         assert_eq!(i.h(), 51);
//     }
//
//     #[test]
//     fn rect_roundtrip_world_screen() {
//         let original = Coordinate::<World, f64>::rect(42.0, 17.0, 100.0, 80.0, ctx());
//         let s: Coordinate<Screen, f64> = original.into();
//         let back: Coordinate<World, f64> = s.into();
//         assert!((back.x() - 42.0).abs() < 1e-9);
//         assert!((back.y() - 17.0).abs() < 1e-9);
//         assert!((back.w() - 100.0).abs() < 1e-9);
//         assert!((back.h() - 80.0).abs() < 1e-9);
//     }
// }
