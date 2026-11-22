#!/usr/bin/env bash
# Proves a built "Automata Playground.app" is self-contained, signed and starts (SPEC DONE-6).
#
#   scripts/check-bundle.sh ["path/to/Automata Playground.app"] [--no-window]
#
# 1. Static: ad-hoc signature verifies (the app and every binary in Contents/MacOS), Info.plist
#    lints and has the expected keys (identifier, executable, version from Cargo, icon, macOS 14,
#    high resolution, education category, .rle / .cells document types), the icon, licences and
#    build-info.json are there, the MCP server is bundled once the ap-mcp crate exists, and no
#    binary or bundled text file contains a path of this machine ($HOME, the checkout).
# 2. Finder: with the bundle registered in LaunchServices (the checked bundle itself, as Finder
#    does when it shows the folder: LaunchServices ignores apps under the temporary folder), the
#    app is offered for a .rle and a .cells file (Open With).
# 3. Smoke runs (once the app has `--smoke <seconds>`; SKIP until then): the bundle is copied
#    out of the checkout and run under sandbox-exec with the whole lab-series folder (this
#    checkout, labkit, the sister apps) denied and the bundle read-only, from a scratch working
#    directory: `automata-playground --smoke 3` must exit 0 within the timeout and print a line
#    starting with `SMOKE ` (e.g. `SMOKE first_frame_ms=412 frames=180 …`) — headless
#    (AUTOMATA_HEADLESS=1) and, unless --no-window, in a real window; then the same through
#    LaunchServices (`open -n … --args --smoke 3`). No output line may mention the checkout.
# 4. Opening a pattern (TRAY-9, unless --no-window; SKIP without a window server): `open -n -a
#    <copy> glider.rle` hands the file over as the Apple Event odoc; the app must log
#    `opened pattern <path>` (it arms the pattern on the Tray as a stamp), then quit on SIGTERM.
set -euo pipefail
cd "$(dirname "$0")/.."
repo="$(pwd -P)"
export PATH="$HOME/.cargo/bin:$PATH"
name="Automata Playground"
bundle_id="dev.labseries.AutomataPlayground"
app="$repo/dist/$name.app"
window=1
for a in "$@"; do
  case "$a" in
    --no-window) window=0 ;;
    -h|--help) sed -n '2,22p' "$0"; exit 0 ;;
    *) app="$a" ;;
  esac
done
[ -d "$app" ] || { echo "no app at $app (run scripts/bundle-macos.sh)" >&2; exit 2; }
app="$(cd "$app" && pwd -P)"

failures=0
skips=0
ok() { printf '  ok    %s\n' "$*"; }
bad() { printf '  FAIL  %s\n' "$*"; failures=$((failures + 1)); }
skip() { printf '  SKIP  %s\n' "$*"; skips=$((skips + 1)); }
section() { printf '\n\033[1m%s\033[0m\n' "$*"; }

# ------------------------------------------------------------------ static
section "Static checks ($app)"
contents="$app/Contents"
res="$contents/Resources"
bin="$contents/MacOS/automata-playground"
plist="$contents/Info.plist"

if codesign --verify --deep --strict "$app" 2>/dev/null; then ok "code signature (ad hoc) verifies"; else bad "codesign --verify --deep --strict"; fi
for b in "$contents"/MacOS/*; do
  if codesign --verify --strict "$b" 2>/dev/null; then :; else bad "$(basename "$b") is not validly signed"; fi
done
if plutil -lint "$plist" >/dev/null; then ok "Info.plist lints"; else bad "Info.plist does not lint"; fi

meta="$(cargo metadata --no-deps --format-version 1 2>/dev/null || echo '{"packages":[]}')"
want_version=$(python3 -c '
import json, sys
print(next((p["version"] for p in json.load(sys.stdin)["packages"] if p["name"] == "automata-playground"), "?"))' <<<"$meta")
want_mcp=$(python3 -c '
import json, sys
print(any(t["name"] == "automata-mcp" and "bin" in t["kind"] for p in json.load(sys.stdin)["packages"] for t in p["targets"]))' <<<"$meta")

for kv in "CFBundleIdentifier=$bundle_id" "CFBundleName=$name" "CFBundleExecutable=automata-playground" \
          "CFBundlePackageType=APPL" "CFBundleShortVersionString=$want_version" \
          "CFBundleIconFile=$name" "LSMinimumSystemVersion=14.0" "NSHighResolutionCapable=true" \
          "LSApplicationCategoryType=public.app-category.education"; do
  k="${kv%%=*}"; v="${kv#*=}"
  got=$(plutil -extract "$k" raw "$plist" 2>/dev/null || echo missing)
  if [ "$got" = "$v" ]; then ok "$k = $v"; else bad "$k = $got (want $v)"; fi
done
build=$(plutil -extract CFBundleVersion raw "$plist" 2>/dev/null || echo missing)
if [[ "$build" =~ ^[0-9]+$ ]]; then ok "CFBundleVersion = $build (build number)"; else bad "CFBundleVersion = $build (want a build number)"; fi
# .rle and .cells: imported UTIs with the extension, and a document type naming each UTI
doc_types=$(plutil -convert json -o - "$plist" | python3 -c '
import json, sys
p = json.load(sys.stdin)
utis = {}
for d in p.get("UTImportedTypeDeclarations", []) + p.get("UTExportedTypeDeclarations", []):
    for ext in d.get("UTTypeTagSpecification", {}).get("public.filename-extension", []):
        utis[ext] = d.get("UTTypeIdentifier")
handled = {u for d in p.get("CFBundleDocumentTypes", []) for u in d.get("LSItemContentTypes", [])
           if d.get("CFBundleTypeRole") in ("Viewer", "Editor")}
print(" ".join(e for e in ("rle", "cells") if utis.get(e) in handled))')
if [ "$doc_types" = "rle cells" ]; then
  ok "document types: .rle and .cells (imported UTIs, Viewer)"
else
  bad "document types declared for: '${doc_types}' (want .rle and .cells)"
fi

icon="$res/$name.icns"
if [ -s "$icon" ]; then
  px=$(sips -g pixelWidth "$icon" 2>/dev/null | sed -n 's/.*pixelWidth: //p')
  if [ "${px:-0}" -ge 1024 ]; then ok "icon: $name.icns (largest ${px} px)"; else bad "icon: $name.icns largest image is ${px:-?} px (want 1024)"; fi
else
  bad "missing Resources/$name.icns"
fi
missing=0
for f in licenses/Inter-OFL.txt licenses/JetBrainsMono-OFL.txt licenses/SpaceGrotesk-OFL.txt \
         licenses/THIRD-PARTY.md build-info.json; do
  [ -s "$res/$f" ] || { bad "missing Resources/$f"; missing=1; }
done
[ $missing -eq 0 ] && ok "Resources: font licences (OFL ×3), THIRD-PARTY.md ($(grep -cE '^\| [^|]+ \| [0-9]' "$res/licenses/THIRD-PARTY.md") crates), build-info.json"
info_version=$(python3 -c 'import json,sys; print(json.load(open(sys.argv[1]))["version"])' "$res/build-info.json" 2>/dev/null || echo "?")
if [ "$info_version" = "$want_version" ]; then ok "build-info.json: $(tr -s ' ' < "$res/build-info.json")"; else bad "build-info.json version $info_version (want $want_version)"; fi

if [ -x "$contents/MacOS/automata-mcp" ]; then
  ok "MCP server bundled: Contents/MacOS/automata-mcp"
elif [ "$want_mcp" = True ]; then
  bad "the workspace builds automata-mcp but the bundle has no Contents/MacOS/automata-mcp"
else
  skip "MCP server: no automata-mcp binary in the workspace yet (ap-mcp crate)"
fi
ok "architectures: $(lipo -archs "$bin" 2>/dev/null || echo '?')"

machine_paths=0
for b in "$contents"/MacOS/*; do
  # each hit with a little context (strings runs can be long)
  hits=$(strings -a "$b" | H="$HOME" R="$repo" perl -ne 'while (/(.{0,24})((?:\Q$ENV{H}\E|\Q$ENV{R}\E)\S{0,64})/g) { print "…$1$2…\n" }' | sort -u | head -5 || true)
  if [ -n "$hits" ]; then
    bad "$(basename "$b") contains paths of this machine:"
    while IFS= read -r l; do printf '        %s\n' "$l"; done <<<"$hits"
    machine_paths=1
  fi
done
[ $machine_paths -eq 0 ] && ok "no binary contains a path under \$HOME or the checkout"
if grep -rIlF -e "$repo" -e "$HOME/" "$res" "$plist" >/dev/null 2>&1; then
  bad "bundled files mention this machine: $(grep -rIlF -e "$repo" -e "$HOME/" "$res" "$plist" | head -3 | tr '\n' ' ')"
else
  ok "no bundled text file mentions the checkout or \$HOME"
fi

# ------------------------------------------------------------------ work on a copy outside the checkout
scratch="$(mktemp -d "${TMPDIR:-/tmp}/automata-check.XXXXXX")"
scratch="$(cd "$scratch" && pwd -P)"
copy="$scratch/$name.app"
lsregister=/System/Library/Frameworks/CoreServices.framework/Frameworks/LaunchServices.framework/Support/lsregister
cleanup() {
  pkill -f "$copy/Contents/MacOS/automata-playground" 2>/dev/null || true
  [ "${KEEP:-0}" = 1 ] || rm -rf "$scratch"
}
trap cleanup EXIT
cp -R "$app" "$copy"
mkdir -p "$scratch/cwd"

# ------------------------------------------------------------------ Finder: Open With
section "Finder document types (LaunchServices)"
"$lsregister" -f "$app" >/dev/null 2>&1 || true
# shellcheck disable=SC2016  # RLE run-length syntax, not shell expansions
printf '#N Glider\nx = 3, y = 3, rule = B3/S23\nbo$2bo$3o!\n' > "$scratch/glider.rle"
printf '!Name: Glider\n.O.\n..O\nOOO\n' > "$scratch/glider.cells"
for f in glider.rle glider.cells; do
  can=""
  for _ in $(seq 1 30); do
    can=$(osascript -l JavaScript -e '
      ObjC.import("AppKit");
      function run(argv) {
        var urls = $.NSWorkspace.sharedWorkspace.URLsForApplicationsToOpenURL($.NSURL.fileURLWithPath(argv[0]));
        var ids = [];
        for (var i = 0; i < urls.count; i++) {
          var b = $.NSBundle.bundleWithURL(urls.objectAtIndex(i));
          if (b && !b.isNil() && b.bundleIdentifier && !b.bundleIdentifier.isNil()) ids.push(ObjC.unwrap(b.bundleIdentifier));
        }
        return ids.indexOf(argv[1]) >= 0 ? "yes" : "no";
      }' "$scratch/$f" "$bundle_id" 2>/dev/null || true)
    [ "$can" = yes ] && break
    sleep 0.2
  done
  if [ "$can" = yes ]; then ok "Open With offers $name for ${f#glider}"; else bad "$name is not offered for ${f#glider} files (Open With)"; fi
done

# ------------------------------------------------------------------ smoke runs
section "Smoke runs (sandboxed: the lab-series folder denied, the bundle read-only)"
# (grep without -q reads all of strings' output: with pipefail, -q's early exit is a SIGPIPE)
if ! strings -a "$bin" | grep -F -- '--smoke' >/dev/null; then
  skip "the app has no --smoke mode yet (see PROGRESS.md Requests); launch checks skipped"
else
  lab_series="$(dirname "$repo")"
  profile="$scratch/deny-lab-series.sb"
  cat > "$profile" <<SB
(version 1)
(allow default)
(deny file-read* file-write* (subpath "$lab_series"))
(deny file-write* (subpath "$copy"))
SB
  smoke() { # smoke <label> <timeout s> [VAR=value…]: run the copy with --smoke 3 under the sandbox
    local label="$1" secs="$2"; shift 2
    local log status=0 line
    log="$scratch/smoke-$(tr -c 'a-zA-Z0-9\n' '-' <<<"$label").log"
    ( cd "$scratch/cwd" && env -u CARGO_MANIFEST_DIR -u AUTOMATA_MCP_SOCKET RUST_LOG=info "$@" \
        perl -e 'alarm shift; exec @ARGV or die "exec: $!"' "$secs" \
        sandbox-exec -f "$profile" "$copy/Contents/MacOS/automata-playground" --smoke 3 ) \
        > "$log" 2>&1 || status=$?
    line=$(grep '^SMOKE ' "$log" | tail -1 || true)
    smoke_mode=$(sed -n 's/.* mode=\([a-z]*\).*/\1/p' <<<"$line")
    if [ $status -ne 0 ] || [ -z "$line" ]; then
      bad "$label (exit $status$([ $status -eq 142 ] && echo ", killed after ${secs} s")): ${line:-no SMOKE line}"
      sed 's/^/        /' "$log" | tail -15
    elif grep -qF "$repo" "$log"; then
      bad "$label: output mentions the checkout: $(grep -F "$repo" "$log" | head -2)"
    elif grep -qi "operation not permitted" "$log"; then
      bad "$label: hit the sandbox: $(grep -i "operation not permitted" "$log" | head -2)"
    else
      ok "$label"
      echo "        $line"
    fi
  }
  smoke "headless (AUTOMATA_HEADLESS=1), 3 s" 15 AUTOMATA_HEADLESS=1
  window_mode=""
  if [ $window -eq 1 ]; then
    smoke "in a real window (wgpu), 3 s" 20
    window_mode="$smoke_mode"
  fi

  if [ $window -eq 1 ]; then
    section "Through LaunchServices (open -n … --args --smoke 3)"
    log="$scratch/open.log"
    : > "$log"
    open -n --stdout "$log" --stderr "$log" "$copy" --args --smoke 3 || bad "open failed"
    line=""
    for _ in $(seq 1 200); do
      line=$(grep '^SMOKE ' "$log" | tail -1 || true)
      [ -n "$line" ] && break
      sleep 0.1
    done
    if [ -n "$line" ]; then ok "open → SMOKE line"; echo "        $line"; else bad "no SMOKE line within 20 s of \`open\` (log: $(tail -3 "$log" | tr '\n' ' '))"; fi
    for _ in $(seq 1 50); do pgrep -f "$copy/Contents/MacOS/automata-playground" >/dev/null || break; sleep 0.1; done
    if pgrep -f "$copy/Contents/MacOS/automata-playground" >/dev/null; then bad "the app was still running 5 s after its smoke run"; else ok "exited by itself"; fi

    section "Opening a pattern (Apple Event odoc: open -a … glider.rle)"
    if [ "$window_mode" != window ]; then
      skip "no window server (the window smoke run ran ${window_mode:-no} mode); open -a needs one"
    else
      # the real app, not a smoke run: its own HOME (no saved state, no files of the user's) and
      # no MCP bridge (the user's running app may own the socket)
      log="$scratch/open-pattern.log"
      : > "$log"
      mkdir -p "$scratch/home"
      open -n -F --env HOME="$scratch/home" --env AUTOMATA_MCP_SOCKET=off --env RUST_LOG=info \
        --stdout "$log" --stderr "$log" -a "$copy" "$scratch/glider.rle" || bad "open -a failed"
      opened=""
      for _ in $(seq 1 150); do
        opened=$(grep -F "opened pattern" "$log" | grep -F "glider.rle" | head -1 || true)
        [ -n "$opened" ] && break
        sleep 0.1
      done
      if [ -n "$opened" ]; then
        ok "open -a $name.app glider.rle arms it on the Tray (${opened##*] })"
      else
        bad "the app did not open the pattern it was handed (log: $(tail -3 "$log" | tr '\n' ' '))"
      fi
      if grep -qF "$repo" "$log"; then bad "the app log mentions the checkout"; fi
      pid=$(pgrep -f "$copy/Contents/MacOS/automata-playground" | head -1 || true)
      if [ -n "$pid" ]; then
        kill -TERM "$pid" 2>/dev/null || true
        for _ in $(seq 1 50); do kill -0 "$pid" 2>/dev/null || break; sleep 0.1; done
        if kill -0 "$pid" 2>/dev/null; then kill -KILL "$pid" 2>/dev/null || true; bad "the app did not quit on SIGTERM"; else ok "quit"; fi
      fi
    fi
  fi
fi

echo
if [ $failures -eq 0 ]; then
  note=""; [ $skips -gt 0 ] && note=" ($skips skipped)"
  echo "bundle check passed$note: $(du -sh "$app" | cut -f1) $(basename "$app")"
else
  echo "bundle check: $failures failure(s), $skips skipped"
  exit 1
fi
