# app_launcher

Application launcher overlay UI for the y5 compositor. Sits in the same
shape as the other y5 UI crates: implements `IcedUi`, talks to the
compositor through a `Message` enum with both UI-internal and
compositor-handled variants.

This crate owns only the **UI logic**. It does not open a surface, run
an event loop, spawn processes, or place windows. All of that stays in
the compositor.

## Surface area

```rust
use app_launcher::{Application, Direction, Launcher, LauncherMessage};
```

- [`Launcher`] — the UI state. Holds the app list, the search query,
  cursor, mode. Implements `IcedUi<Message = LauncherMessage>`.
- [`LauncherMessage`] — the message enum the compositor dispatches and
  reads.
  - **Compositor-handled** (the compositor reacts to these; the UI's
    `update` arm is a no-op):
    - `Launch { id, bin, args, direction }`
    - `Exit`
  - **UI-internal** (the compositor dispatches these into `update`):
    - `Event(iced_core::Event)` — every iced event the overlay receives.
      Built from xkb via the runtime's `keyboard_event` helper.
    - `Tick` — optional periodic refresh of the frecency ranking.
    - `SetApps(Arc<Vec<Application>>)` — replace the app list.

## Lifecycle

```rust,ignore
// At overlay open:
let mut launcher = Launcher::new(initial_app_list);

// Each turn of the compositor's loop:
//
// 1. Dispatch incoming events into the iced runtime.
//    Wrap xkb-derived events as LauncherMessage::Event(_).
// 2. After dispatch, drain compositor-facing emissions:
for msg in launcher.take_emissions() {
    match msg {
        LauncherMessage::Launch { id, bin, args, direction } => {
            spawn_and_place(bin, args, direction);
        }
        LauncherMessage::Exit => {
            close_overlay();
        }
        _ => unreachable!("take_emissions yields only Launch / Exit"),
    }
}
```

Key events arrive as `iced_core::Event` (the compositor already builds
these from xkb for text-input compatibility); the launcher reads them
inside `LauncherMessage::Event(_)` and reacts. When a keypress would
emit a compositor-handled message — Enter→arrow combo, or Esc-at-idle —
it goes into `pending_emissions`, drained by `take_emissions()`.

## Behaviour

- **Banner**: full-width black banner, vertical content centred. Icons
  are 96 px, cells 116 px.
- **Default view**: icons only, sorted by frecency. The title of the
  current selection is shown beneath the row.
- **Search**: typing any printable character triggers a centred search
  chip overlay on top of the icon row. Backspace edits. Escape clears
  the query (one press) and then emits `Exit` (second press).
- **Navigation**:
  - Left/Right: move the selection cursor along the icon row.
  - Enter: focus the selected icon. The banner shows
    `← ↑ ↓ →   pick direction   ·   Esc cancel`.
  - In focused mode, an arrow key emits `Launch { …, direction }`
    immediately followed by `Exit`.
  - Escape from focused mode returns to browse without losing the query.

## Search algorithm

Ported from the user-supplied TypeScript reference, generalised from a
tree to the flat `(title, bin_filename)` pair. Subsequence matcher that
rewards contiguous runs (quadratic), gives a small early-position
anchor bonus, and a length bonus so shorter haystacks win for equal
match quality. Bin-name hits are dampened 0.85× vs title hits.

Default ordering uses Firefox-style frecency: log-compressed usage
count combined with a 14-day-halflife exponential recency decay.
Search ranking is `match_score + 0.15 × frecency`, so relevance
dominates and frecency only breaks ties.

See `src/search.rs` for the full implementation and tests.

## Style

`pub use style as theme;` exposes the colour palette and sizing
constants so the compositor can match the banner's look in any chrome
it draws around the overlay surface.
