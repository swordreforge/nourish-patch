<!--
  Status banner. The coverage badge is self-hosted: the docs/Pages build generates
  coverage.svg + a per-crate report under /docs — no third-party service.
-->
[![CI](https://github.com/snowies/nourish/actions/workflows/ci.yml/badge.svg)](https://github.com/snowies/nourish/actions/workflows/ci.yml)
[![Docs](https://github.com/snowies/nourish/actions/workflows/docs.yml/badge.svg)](https://github.com/snowies/nourish/actions/workflows/docs.yml)
[![Coverage](https://nourish.snowies.com/docs/coverage.svg)](https://nourish.snowies.com/docs/coverage/)
[![Release](https://img.shields.io/github/v/release/snowies/nourish?sort=semver)](https://github.com/snowies/nourish/releases)
[![License: MIT OR Apache-2.0](https://img.shields.io/badge/license-MIT%20OR%20Apache--2.0-blue.svg)](#license)

# y5

`y5` is a Wayland compositor written in Rust, built on vendored forks of `smithay`
(Wayland), `bevy` + `wgpu` + `naga_oil` (rendering) and `iced` + `cryoglyph` (UI), all
patched in-tree under `vendor/`. The shipped binary is **`y5_compositor`**.

## Repository map

| Path | What |
| --- | --- |
| `compositor*/` | the independent top-level Cargo workspaces (core compositor, support libs, loader, rpc, monitor, …) |
| `environment/` | build / run / deploy scripts (containerized dev loop, release builds) |
| `document/` | reference guides — **`TRANSFORM.md`** (coordinate model) and **`LOGGING.md`** (the structured logging system) |
| `ci/` | the CI/CD pipeline (portable scripts + the Fedora 44 CI image) |
| `vendor/` | patched dependencies (treat as part of the codebase) |
| `CLAUDE.md` | conventions + discovery commands — **start here when working in the tree** |

## Install (end users)

On Fedora 44, install the prebuilt compositor with one command (runtime libraries
only — no toolchain). See **[`compositor.installer/INSTALL.md`](compositor.installer/INSTALL.md)**:

```bash
curl -fsSL https://nourish.snowies.com/release/latest/fedora44/package.tar.gz | tar -xz && y5-install/install.sh
```

## Build & run

There is no root `Cargo.toml`: each `compositor*/` directory is its own workspace. Build
the core compositor with `cd compositor && cargo build`; build / run / deploy the actual
binary through the scripts in `environment/` (see `environment/README.md`). Adding or
renaming a crate uses the `add-crate` skill and requires re-running `./link.all.sh`.

## CI/CD

Lint + build + test + **full-codebase coverage (dead code included)**, generated docs, an
LLM doc-review on PRs, a develop→master promotion flow, and release artifacts on tag —
running on both GitHub Actions and GitLab CI from one set of portable scripts. See
**[`ci/README.md`](ci/README.md)**.

## Website

The public-facing site (branded **Nourish**) lives in **[`document/website/`](document/website/)**
and is published to <https://nourish.snowies.com> via GitHub Pages; the generated rustdoc +
coverage site is served under `/docs`.

## License

Licensed under either of

- Apache License, Version 2.0 ([LICENSE-APACHE](LICENSE-APACHE) or
  <https://www.apache.org/licenses/LICENSE-2.0>)
- MIT license ([LICENSE-MIT](LICENSE-MIT) or <https://opensource.org/licenses/MIT>)

at your option. Unless you explicitly state otherwise, any contribution intentionally
submitted for inclusion in this project by you, as defined in the Apache-2.0 license,
shall be dual licensed as above, without any additional terms or conditions.
