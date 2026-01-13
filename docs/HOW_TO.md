# How To: Inspect and Optimize Monaco MBTiles

Purpose: Provide a reproducible, low-context workflow for inspecting and optimizing a sample tileset.

## Prerequisites

- Prepare an MBTiles file (e.g., a Monaco tileset).
- Optional: prepare a Mapbox/MapLibre style JSON if you want style-based optimize.
- Build the CLI: `cargo build --release` (or use `cargo run` in dev).

## Inspect (context-saving)

Set an environment variable for your tileset, then run a minimal summary + layer list with sampling:

```bash
export MBTILES_PATH=path/to/monaco.mbtiles

cargo run -- inspect "$MBTILES_PATH" \
  --stats summary,layers \
  --fast
```

Notes:
- `--stats summary,layers` keeps the output concise.
- `--fast` uses sampling to reduce runtime and output volume.
- For machine-readable output, add `--output ndjson`.

## Optimize

If the output file already exists, remove it first:

```bash
export OUTPUT_MBTILES_PATH=./tmp/monaco.optimized.mbtiles
rm -f "$OUTPUT_MBTILES_PATH"
```

Run optimize with a style filter (optional but recommended when you have a style file):

```bash
export STYLE_PATH=path/to/style.json

cargo run -- optimize "$MBTILES_PATH" \
  --style "$STYLE_PATH" \
  --output "$OUTPUT_MBTILES_PATH"
```

The command prints a summary of removed features and layers.

## Optional: Verify optimized output

```bash
cargo run -- inspect "$OUTPUT_MBTILES_PATH" --stats summary,layers --fast
```
