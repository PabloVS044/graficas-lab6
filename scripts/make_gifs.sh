#!/usr/bin/env bash
# Convierte los cuadros de docs/frames en GIF. Requiere ffmpeg.
# Uso: cargo run --release -- --export-clips && scripts/make_gifs.sh
set -euo pipefail

for folder in docs/frames/*/; do
    name=$(basename "$folder")
    ffmpeg -y -loglevel error -framerate 15 -i "$folder%04d.png" \
        -vf "scale=400:-1:flags=lanczos,split[a][b];[a]palettegen=max_colors=96:stats_mode=diff[p];[b][p]paletteuse=dither=bayer:bayer_scale=4" \
        "docs/$name.gif"
    echo "docs/$name.gif"
done
