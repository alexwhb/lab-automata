#!/usr/bin/env bash
# scripts/screenshots.sh — renders the README's screenshots: every screen at 1440 px in the
# state its mockup reference shows (the Tray running at gen 30, the Loom at row 47, …), with
# the real app in egui_kittest (wgpu, 1×), into docs/screenshots/<screen>.png.
set -euo pipefail

repo="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$repo"
export PATH="$HOME/.cargo/bin:$PATH"
export CARGO_TARGET_DIR="${CARGO_TARGET_DIR:-target/screenshots}"

AP_SCREENSHOTS=1 cargo test -p automata-playground --test compare -- --nocapture readme_screenshots \
  | grep -E '^screenshot:' || true
