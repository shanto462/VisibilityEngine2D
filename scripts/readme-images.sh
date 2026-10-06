#!/usr/bin/env bash
# Rebuilds the README images in assets/ by driving the app's --screenshot mode.
#
# Usage: scripts/readme-images.sh
#
# The app opens a window for each image, so keep it in front until the script ends.
# Images are captured at the display's scale factor; on a Retina screen they are
# downscaled to the window size with sips (macOS) so the files stay small.

set -euo pipefail
cd "$(dirname "$0")/.."

cargo build --release --locked -p visibility-app
app=target/release/visibility-engine-2d

shot() {
    local out=$1
    shift
    "$app" "$@" --screenshot "$out" &
    local pid=$!
    # A window hidden behind others may never get a frame; do not wait forever.
    (sleep 30 && kill "$pid" 2>/dev/null && echo "timed out: $out" >&2) &
    local watchdog=$!
    wait "$pid"
    kill "$watchdog" 2>/dev/null || true
    wait "$watchdog" 2>/dev/null || true
}

shot assets/hero.png --theme light --viewer 1375,975 --range 400 --window 1440x900
shot assets/gallery/shadow-dark.png --theme dark --viewer 1575,575 --center 1400,600 --range 400 --rays \
    --hide-panel --window 1200x800
shot assets/gallery/frustum.png --theme light --mode frustum --viewer 1375,975 --fov 75 --direction 30 \
    --range 450 --hide-panel --window 1200x800
shot assets/gallery/occlusion.png --theme light --mode occlusion --rays --viewer 1575,575 --center 1400,600 \
    --fov 120 --direction 150 --range 450 --hide-panel --window 1200x800
shot assets/gallery/large-scene.png --theme light --obstacles 100000 --world 28000 --viewer 13100,15200 \
    --range 1500 --zoom 0.3 --window 1440x900

if command -v sips >/dev/null; then
    for img in assets/hero.png assets/gallery/*.png; do
        width=$(sips -g pixelWidth "$img" | awk '/pixelWidth/ {print $2}')
        if [ "$width" -gt 1600 ]; then
            sips --resampleWidth $((width / 2)) "$img" >/dev/null
        fi
    done
fi
echo "Done. Images are in assets/."
