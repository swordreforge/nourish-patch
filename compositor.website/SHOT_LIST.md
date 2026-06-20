# Nourish — Shot List (v1)

General rules for every take: fresh session, 125%+ font scale in terminals and
editors (phone viewers), cursor speed slow and deliberate, no notifications,
record at native resolution, export .webm (VP9) + .mp4 fallback. For grid-card
loops, also export a 5–8s cropped excerpt from the matching long take that
starts and ends in the same visual state.

Impersonal demo content used throughout:
- The Nourish repo open in Zed + a terminal (dogfooding, zero personal data)
- Blender with a simple scene (the default cube, lightly dressed, is fine)
- YouTube playing a Blender Foundation open movie (Big Buck Bunny / Sintel)
- Wikipedia "Aurora" article, OpenStreetMap, GIMP/Inkscape with a snowflake

---

## SHOT 1 — HERO MONTAGE (30–40s) · for the hero video slot
One continuous take. The arc: zoom in → work → zoom out → travel → rest.

1. (0–6s) Start mid-zoom on the canvas. Pan smoothly to a YouTube window
   playing Big Buck Bunny; zoom IN until it fills most of the frame —
   linger 2s so the crisp upscaling reads on camera.
2. (6–14s) Pan down/right to the Blender cluster: one main Blender window,
   two small stacked Blender windows (top view + wireframe).
3. (14–24s) Select the three Blender windows, group them, name the group
   ("Render"), then fullscreen the main window *within* the group.
4. (24–32s) Zoom ALL the way out: the whole canvas is revealed — YouTube
   corner, the Render group, and a zone holding Zed + terminal with the
   Nourish repo.
5. (32–40s) One zone jump (snap to the Zed zone, beat, snap back to wide),
   then let the camera rest on the full canvas. End frame = the thesis shot;
   also screenshot it for the OG image.

## SHOT 2 — GROUPS (15–20s) · highlight section 1
1. Spawn 4–5 mixed windows scattered untidily (terminal, Wikipedia, GIMP,
   image viewer).
2. Box-select them; run align + distribute — the satisfying snap is the
   money moment, let it breathe for a beat.
3. Group them, type a name ("Reading").
4. Collapse the group: windows vanish, the name chip remains. Hold 2s.
   (Optional tail: expand it again — makes a better seamless loop.)

## SHOT 3 — PLACEHOLDERS / SELF-HEALING (20–25s) · highlight section 2
1. A terminal sits on the canvas, cd'ed deep into the Nourish repo
   (`src/render/` — path visible in prompt). Kill it (close, or `kill -9`
   from another terminal for the "crash" framing).
2. The placeholder appears at the exact same size — zoom slightly so the
   silhouette reads.
3. Restore it: terminal returns at the same geometry, prompt shows the SAME
   cwd. Type `pwd`, enter, hold on the output.
4. Repeat fast-cut with Chrome: three recognizable tabs (Wikipedia /
   YouTube / OpenStreetMap), close, restore, tabs return.

## SHOT 4 — NAVIGATION (12–15s) · grid card
Layout: Zed (center) with the Nourish repo, browser docs (right),
terminal (below). All keyboard, no mouse:
1. Editing in Zed → hop focus RIGHT to docs, scroll once.
2. Hop DOWN to terminal, run `cargo build` (or any short command).
3. Hop back UP-LEFT to Zed and keep typing. The point on camera: hands
   never leave the keyboard, eyes never hunt.

## SHOT 5 — ZONES (15–20s) · highlight section 3
1. Wide view of a canvas with 2 obvious clusters.
2. Jump between two preset zones — two snappy jumps, small pause between.
3. Then create one: box-select a cluster of windows, declare it a zone,
   name it ("Lab"). Jump away and jump back to it to prove it stuck.

## SHOT 6 — CAPTURE (15–20s) · grid card
1. Full-screen capture: trigger it, then open the saved image so viewers
   see the result.
2. Per-window capture of the Blender window; open result.
3. Canvas capture: zoom to a mid level, capture, show that the output
   matches the zoomed framing. (This one is unique to Nourish — give it
   the most screen time of the three.)

---

Cutting plan: Shots 2, 3, 5 are the three home-page highlight loops.
Shots 4 and 6 plus excerpts of 1 (zoom moment, group moment) become the
mini grid-card loops. Shot 1 is the hero. The end frame of Shot 1 doubles
as the OG/social image.
