# y5.compositor.settings

Interactive companion tool that authors the compositor's settings file,
`~/.config/y5.compositor/settings.json` (override the location with
`--config-file=<path>`).

The compositor reads **all** of its configuration from this file and has no
defaults — it panics on a missing or partial file. This tool always writes a
**complete** file (every field), so a generated file is always loadable.

## Schema, without drift

This is a standalone workspace (like the other `developer.tool` tools — not in
`link.all.sh`), but it **path-depends on the exact same `config.base` crate** the
compositor parses. There is one `Environment` struct, compiled twice; the tool and
the compositor can never disagree on the schema.

## Usage

```
cargo run -- [OPTIONS]

  --config-file=<PATH>   Write to PATH instead of the default location.
  --write-default        Non-interactive: write the canonical default settings.
  -h, --help             Show help.
```

Cargo target names can't contain `.`, so the binary builds as
`y5-compositor-settings`. Installers/run-scripts expose it under the command name
`y5.compositor.settings` (a symlink). For first-run/unattended provisioning, call
`y5-compositor-settings --write-default` to drop a valid file.
