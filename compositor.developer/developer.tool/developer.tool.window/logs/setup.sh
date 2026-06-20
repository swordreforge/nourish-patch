#!/usr/bin/env bash
# Install the Fedora system libraries Tauri needs to build/run the GUI. Run once, then:
#   npm install && npm run tauri dev
set -e

sudo dnf install -y \
    webkit2gtk4.1-devel \
    libsoup3-devel \
    gtk3-devel \
    librsvg2-devel \
    libappindicator-gtk3-devel \
    patchelf \
    protobuf-compiler
