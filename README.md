<div align="center">

<img src="compositor.website/assets/favicon.svg" alt="Nourish" width="84" height="84">

# Nourish

<img src="compositor.website/assets/hero-sphere.png" alt="The Nourish world — one endless canvas" width="300">

<br><br>

[![CI](https://github.com/y5-snowies/nourish/actions/workflows/ci.yml/badge.svg)](https://github.com/y5-snowies/nourish/actions/workflows/ci.yml)
[![Coverage](https://nourish.snowies.com/docs/coverage.svg)](https://nourish.snowies.com/docs/coverage/)
[![Release](https://img.shields.io/github/v/release/y5-snowies/nourish?sort=semver)](https://github.com/y5-snowies/nourish/releases)
[![License: MIT OR Apache-2.0](https://img.shields.io/badge/license-MIT%20OR%20Apache--2.0-blue.svg)](#license)

### A simple next generation OS for work.

<video src="https://nourish.snowies.com/assets/hero.mp4" poster="compositor.website/assets/hero-sphere.png" controls muted loop width="720">
  <a href="https://nourish.snowies.com">▶ Watch the intro at nourish.snowies.com</a>
</video>

**[nourish.snowies.com](https://nourish.snowies.com)**  ·  [Guide](https://nourish.snowies.com/guide)  ·  [Discord](https://discord.gg/kasec5bYb)

</div>

---

Nourish is a Linux desktop that doesn't limit you to your screen size.

It's free, open source, and stable to be used as daily driver.
It's performant, and renders using Vulkan. optionally, you can set automatic fallback or explicitly select Gles on systems where Vulkan is not supported.

It fully supports NVIDIA and cards that use Mesa drivers such as Intel and AMD.

## Features

- It is much better than any other desktop environments, and once you start using it, you can't go back.
- Very stable- based on many years of experience of coding, it is carefully designed to avoid faults and performance issues.
- Featuring a viewport where you can zoom and pan so you can have infinite amount of space available to work on.
- Utilizing modern features such as Wayland protocol for fractional scale, complying windows do not get blurried and upscale their texture so that everything remains sharp under different zoom levels.
- Additional non intrusive features make it easy to multitask and work ergonomically across many contexts concurrently.

Visit **[nourish.snowies.com](https://nourish.snowies.com)**  to see how it looks like and the full list of features.

## Install

On Fedora 44, it's one command. You get a prebuilt build, so there's no toolchain to set up:

```bash
curl -fsSL https://nourish.snowies.com/release/latest/fedora44/package.tar.gz | tar -xz && y5-install/install.sh
```

The installer is interactive and safe to re-run. For the full walkthrough see
[`https://nourish.snowies.com/guide.html`](https://nourish.snowies.com/guide.html).

Prefer a pinned build? Every release is also published immutably under its version —
`https://nourish.snowies.com/release/v1.0.0/fedora44/package.tar.gz` — while `latest`
always points at the newest. Browse them on the
[releases page](https://github.com/y5-snowies/nourish/releases).

For any other distribution, please see [`nourish.snowies.com/guide.html`](https://nourish.snowies.com/guide.html) I currently do not publish individual binaries for different distributions and generally recommend using Fedora.
If you are using a different distribution, it is easy to build from source which will link against your distribution system libraries versions automatically. 

## Source

Under the hood the engine is called **`y5`** — a
Wayland compositor written in Rust, standing on patched forks of
[smithay](https://github.com/Smithay/smithay) (Wayland), [bevy](https://bevyengine.org) +
[wgpu](https://wgpu.rs) (rendering), and [iced](https://iced.rs) (interface), all kept
in-tree under `vendor/`.

A thorough guide available [`here`](https://nourish.snowies.com/guide.html).

Note: Y5 was architected and hand-written and only later enchanced with AI. It has a lot of generated code which was pre-directed and reviewed carefully.

```bash
# Build & run nested in your current Wayland session
environment/run-host.sh winit release

# Build the binary for use
environment/build-release.sh system

If you get any errors about missing libraries, these are system libraries that the project links with. 

You can use environment/install-deps.sh to install the requirements on Fedora, and if you are on different distribution you can ask you AI agent which equivalent packages are available for your distribution by feeding it the install-deps.sh script.
```

The conventions, architecture, and discovery commands live in [`CLAUDE.md`](CLAUDE.md) and
[`document/`](document/).

## License

Licensed under either of [Apache License 2.0](LICENSE-APACHE) or
[MIT license](LICENSE-MIT) at your option.
