# compositor.developer.tool — y5 log viewer

A standalone **Tauri 2 + React** desktop app that streams structured logs from the running
compositor over gRPC and shows them in a filterable viewer. It is **outside** the compositor's
cargo graph: it uses none of `vendor/`, defines its own dependencies, and is not part of
`link.all.sh`.

## How it works

```
compositor (compositor.developer log process)
  └─ gRPC LogStream server on  /tmp/y5-compositor-logs.sock
        │  (server-streaming LogRecord)
        ▼
src-tauri (Rust)  ── tonic unix-socket client ──▶  emits Tauri "log" events
        ▼
React frontend (src/) ── live, filterable log table
```

The Rust backend (`src-tauri/src/main.rs`) connects to the socket, streams `LogRecord`s, and
forwards each to the frontend. The frontend (`src/App.tsx`) renders them with filtering by
**level**, **crate** tag, **function** tag, and message text search, plus pause / clear / follow.

## Toolchain

- Frontend: **React + TypeScript**, bundled with **esbuild** (no Vite). Strict `tsconfig` and
  strict type-aware ESLint (`typescript-eslint` strict + stylistic). `npm run typecheck` and
  `npm run lint` (`--max-warnings 0`) are the gates.
- Backend: Tauri 2, tonic/prost (own copy of `proto/logs.proto`).

## Run

```bash
./setup.sh                 # one-time: webkit/gtk/soup system libs (Fedora)
npm install
npm run typecheck && npm run lint
npm run tauri dev          # opens the viewer (nested on the current Wayland session)
```

Start the compositor (with `COMPOSITOR_LOG_LEVEL` set) first, or the viewer will keep retrying
until the socket appears.

> **Rendering note:** on nested / NVIDIA Wayland sessions webkit2gtk shows a blank window
> unless its DMABUF renderer is off. The app sets `WEBKIT_DISABLE_DMABUF_RENDERER=1` itself
> (honoring an existing value). If the window is still blank, also try
> `WEBKIT_DISABLE_COMPOSITING_MODE=1`.
>
> The Tauri **capabilities** file (`src-tauri/capabilities/default.json`) grants the window
> `core:default` — without it the frontend's `listen()` is denied and no logs appear.

## Scripts

| script | what |
| ------ | ---- |
| `npm run dev` | esbuild bundle + watch + serve on `127.0.0.1:1420` (used by `tauri dev`) |
| `npm run build` | `tsc --noEmit` typecheck, then a minified esbuild bundle |
| `npm run typecheck` | strict `tsc --noEmit` |
| `npm run lint` | strict ESLint, zero warnings allowed |
| `npm run tauri dev` / `build` | run / package the desktop app |
