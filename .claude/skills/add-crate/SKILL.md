---
name: add-crate
description: Add a new member crate to a Cargo workspace in this repo using the `y5-template` binary. Use whenever the user asks to add/create/scaffold a new crate, module, or workspace member (e.g. "add a crate under compositor.window", "scaffold action.foo"). Handles the chain-prefix naming convention and non-interactive invocation.
---

# add-crate

Scaffold a new member crate into a Cargo workspace using `y5-template` (on PATH at
`~/.cargo/bin/y5-template`; source in `references/y5-template/`). Do NOT hand-create
`Cargo.toml`/`lib.rs` by hand — use the tool so naming and the template stay correct.

**ALWAYS run `./link.all.sh` from the repo root after creating a crate** (step 5) —
the crate is not wired into the workspaces until you do.

## The naming convention (chain-prefix)

A crate is created **exactly two levels** below a workspace root, and each level's
directory name chains off its parent: `{parent_tail}.{own_segment}`.

```
compositor/                  workspace root  (Cargo.toml [workspace] members glob has ≥2 `*`)
  compositor.action/         L0   (root_tail "compositor" + "action")
    action.window/           L1 = the target directory
      window.{Name}/         L2 = what the tool CREATES
```

- **root_tail** = last dot-segment of the workspace dir name (`compositor` → `compositor`).
- A valid **L0** is `{root_tail}.{seg}`, a valid **L1** is `{seg}.{sub}`.
- The created dir is `{L1_own_segment}.{Name}` and must not already exist.

## Steps

1. **Find valid targets** (the L1 dirs you can add into):
   ```
   y5-template --scan /workspace --list
   ```
   This prints every valid `<workspace> › <L0> › <L1>` with its absolute path.

2. **Create the crate non-interactively.** The picker needs a TTY, so use `--dir`
   (which skips the picker when the dir validates) and pipe the Name to stdin:
   ```
   printf 'NAME\n' | y5-template --dir /workspace/compositor/compositor.action/action.window
   ```
   - `NAME` becomes the crate suffix → creates `window.NAME/`.
   - **Batch:** `printf 'a, b, c\n' | y5-template --dir <L1_DIR>` creates `window.a`,
     `window.b`, `window.c` atomically (aborts if any already exists).
   - **Extra template vars:** if the chosen template declares vars beyond `Name`,
     they are prompted after Name — feed them as additional lines:
     `printf 'NAME\nval1\nval2\n' | y5-template --dir <L1_DIR>`.
     Check first with: `y5-template --template <NAME> --help` is not enough — inspect
     the template under `<workspace>/y5.template/<tpl>/` for `$${var}$$` placeholders.
   - `--template <NAME>` selects a non-default template (default is `default`).

3. **Need a new L0 or L1 first?** The `+ L0/L1` bootstrap only works through the
   interactive picker, so non-interactively just create the chain-prefixed dirs
   yourself, then run step 2:
   ```
   mkdir -p /workspace/compositor/compositor.<L0>/<L0>.<L1>
   printf 'NAME\n' | y5-template --dir /workspace/compositor/compositor.<L0>/<L0>.<L1>
   ```

4. **Verify**: confirm `{L1}.{Name}/` exists with the template files, and that the
   workspace `Cargo.toml` member glob already covers it (it does if the glob is
   `compositor.<L0>/*/*`-style — no manual `members` edit needed).

5. **Relink workspaces — REQUIRED, do not skip.** A new crate is not usable across
   the repo until the workspace path links are regenerated. Run from the repo root:
   ```
   cd /workspace && ./link.all.sh
   ```
   `link.all.sh` runs `workspace.link.js` inside each top-level workspace
   (`compositor`, `compositor.support/support.smithay`, `compositor.background`,
   `compositor.rpc`, `compositor.loader`, `compositor.introspection`,
   `compositor.monitor`). That script discovers each workspace's crates (manual
   Cargo.toml parsing) and rewrites the cross-workspace path dependencies declared
   via each dir's `link.json`. Skipping it leaves the new crate unlinked and
   downstream workspaces won't resolve it. Run it once after a batch — not per crate.

## Template variables (for reference)

For `compositor/compositor.action/action.window` + `Name = handle`:

| variable                      | value                             |
|-------------------------------|-----------------------------------|
| `workspace_name`              | `compositor`                      |
| `L0` / `L1`                   | `action` / `window`               |
| `Name`                        | `handle`                          |
| `fully_qualified_crate_name`  | `compositor_action_window_handle` |
| `fully_qualified_module_name` | `handle`                          |

## Notes

- `--scan` defaults to `$ZED_WORKTREE_ROOT` then cwd; pass `--scan /workspace`
  explicitly when running from elsewhere.
- In Zed, humans use the picker via `alt-n` (default template) / `alt-shift-n`
  (named); that path also pins the L1 of the currently-open file. The CLI `--dir`
  flow above is the equivalent for non-interactive/agent use.
- Full reference: `references/y5-template/README.md`.
