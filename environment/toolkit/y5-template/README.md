# y5-template

Scaffold a new directory from a `y5.template`, with strict chain-prefix
validation, triggered from Zed.

## Why a binary + Zed task + a picker (not a Zed extension, not `$ZED_DIRNAME`)

Two hard Zed constraints shaped this design:

1. **Zed extensions can't do this.** The WASM extension surface is limited to
   language servers, debuggers, themes, slash commands, and context servers —
   no project-panel context menus, no palette commands, no input forms.
2. **Zed task variables can't see the project-panel selection.** Every task
   variable (`$ZED_DIRNAME`, `$ZED_FILE`, …) reflects the active *editor*
   buffer, not the directory highlighted in the project panel. (Confirmed
   behavior; triggering a task with the panel focused still reports the last
   opened file's directory.)

So the target directory is **chosen from an interactive picker** in Zed's
terminal, rather than received from Zed. A keybinding spawns a task that runs
the `y5-template` binary; the binary scans the worktree for valid targets and
lets you fuzzy-filter to the one you want.

Tradeoffs vs. the original spec:
- **Right-click context menu → not possible** in Zed; you get a keybinding.
- **Panel-selection-aware trigger → not possible**; replaced by the picker.
- **GUI input form → terminal prompts.**

## The structure it expects

```
compositor.ui/                  workspace root        (name = "compositor.ui")
  ui.member.1/                  L0   own-segment = "member.1"
    member.1.example/           L1 = the TARGET you pick
      example.{Name}/           L2 = CREATED by the tool
```

Rules (any violation makes a directory ineligible / aborts generation):

- **Workspace root** = a directory whose `Cargo.toml` has a `[workspace]`
  members entry with a two-level glob (e.g. `members = ["member.1/*/*"]`).
  Independent workspaces nested anywhere under the scan root are all found.
- A target sits **exactly 2 levels** below its workspace root (L0, then L1).
- **Chain-prefix convention:** each level's directory name is
  `{parent-tail}.{own-segment}`. The root's tail is the last dot-segment of its
  name (`compositor.ui` → `ui`). So `ui.member.1` is a valid L0, and
  `member.1.example` a valid L1. `member.1` directly under the root (missing
  the `ui.` prefix) is rejected.
- The created dir `{L1}.{Name}` must **not already exist**.

## Variables available to templates

For target `compositor.ui/ui.member.1/member.1.example`, input `Name = crate`:

| variable                      | value                                   |
|-------------------------------|-----------------------------------------|
| `workspace_name`              | `compositor.ui`                         |
| `L0`                          | `member.1`                              |
| `L1`                          | `example`                               |
| `Name`                        | `crate` (raw input; may contain dots)   |
| `fully_qualified_crate_name`  | `compositor_ui_member_1_example_crate`  |
| `fully_qualified_module_name` | `crate` (the Name, dots → `_`)          |

`fully_qualified_crate_name` = workspace name + each level's own-segment +
Name, joined by `.`, then all dots → `_`.

## Template format

- Templates are folders inside a `y5.template` host folder, looked for at the
  **workspace root** and **one directory above it**.
- The default keybinding auto-selects the template named **`default`** (or, if
  there's exactly one, that one). A second keybinding picks a named template.
- A template is a tree of text files. Use `$${variable}$$` anywhere in file
  **contents** or in file/directory **names**. `$${Name}$$` is always required;
  any other `$${var}$$` is prompted for. Non-UTF-8 files are copied verbatim.

## Build & install

```bash
cargo build --release
cp target/release/y5-template ~/.cargo/bin/   # or anywhere on your PATH
```

(On modern toolchains, bump `crossterm` to the latest in Cargo.toml; it's
pinned to 0.27 here only for older-rustc compatibility.)

## Wire into Zed

1. Merge `zed/tasks.json` into your tasks (`zed: open tasks`) or a project
   `.zed/tasks.json`.
2. Merge `zed/keymap.json` into your keymap (`zed: open keymap file`).
   Defaults: `alt-n` (default template), `alt-shift-n` (named template).

Then press `alt-n` anywhere in the project: a picker lists every valid target;
type to filter, arrow-keys to move, Enter to choose, then type `Name` + Enter.

## Picker controls

- Type to fuzzy-filter, `↑/↓` or `Ctrl-N/Ctrl-P` to move, `Enter` to select,
  `Esc`/`Ctrl-C` to cancel.
- If only one target exists it's auto-selected.
- In a non-interactive terminal it falls back to a numbered prompt.

## Bootstrapping new L0/L1 dirs from the picker (`/`)

If you type a query containing `/` into the picker, normal filtering is
suspended and a single `+ create …` command appears. The query must be
exactly `L0/L1` (two non-empty segments separated by one slash); anything
else (`a/b/c`, `/foo`, etc.) shows a non-selectable error so the picker
visibly refuses to act.

For example, in an empty workspace `compositor`, typing `member.1/example`
and pressing Enter ensures these directories exist (mkdir -p semantics):

```
compositor/
  compositor.member.1/
    member.1.example/
```

…and then the normal Name prompt + template flow continues, scaffolding
inside the just-created L1. For a dotted workspace name like `compositor.ui`,
the L0 dir uses only the workspace name's tail (`ui.member.1/`), matching the
chain-prefix rule used everywhere else. Re-running with the same path-spec is
idempotent — existing dirs are reused, not re-created.

This also works in an **empty workspace with no existing L1s**, since the
picker no longer requires base targets — typing the path-spec is the way to
populate an empty workspace.

## Current-L1 pinning

When invoked while editing a file inside a valid L1 (or anywhere nested
beneath it — `src/foo/bar.rs` inside an L2 still counts), the picker pins
that L1 as the first entry, marked with `★ … (current)` and pre-selected.
This makes the common "scaffold a sibling crate to what I'm editing" flow
three keystrokes: `alt-n` → Enter → Name → Enter.

The detection reads `$ZED_FILE` (passed through by the task) and walks up
from its directory until an L1 resolves. If the edited file isn't inside any
L1 — for example you've got the workspace `Cargo.toml` open, or no file at
all — there's no pin and the picker behaves as before.

## Batch creation (comma-separated Name)

Type `a, b, c` at the Name prompt to create three crates in one go:
`{L1}.a`, `{L1}.b`, `{L1}.c`. The behavior is **atomic** — every destination
is checked before any write, so if one of them already exists the *whole*
batch aborts and nothing is created. Duplicate names in the batch are
rejected; empty entries (e.g. `a,,b`) are ignored.

If the template declares extra variables beyond `Name`, they're prompted
**once upfront** and the same values apply to every name in the batch. With
`--open`, only the **last** created crate's file is opened.

## Opening the new file after creating (`--open`)

Pass `--open` and, after a successful create, the tool opens a file from the
new crate in Zed via the `zed` CLI. It tries, first match wins:
`src/<module>.rs`, `<module>.rs`, `src/lib.rs`, `lib.rs` — where `<module>` is
the `fully_qualified_module_name` (the Name, dots → `_`). If none exist, or the
`zed` CLI isn't on PATH, it prints the path instead (the create still
succeeded). Because the file is inside the current project, `zed <path>`
reuses the existing window rather than opening a new one.

Install the `zed` CLI if needed: on macOS run `cli: install cli binary` from
the command palette; on Linux it ships with Zed (binary `zed` or `zeditor`).

## CLI

```
y5-template [--scan <ROOT>] [--template <NAME>] [--open]   # scan + pick + create
y5-template --dir <L1_DIR> [--template <NAME>]    # skip picker if dir validates
y5-template [--scan <ROOT>] --list                # list valid targets
```

`--scan` defaults to `$ZED_WORKTREE_ROOT`, then the current directory.
