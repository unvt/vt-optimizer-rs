# tile-prune

A fast CLI to inspect and prune MBTiles/PMTiles vector tiles. It supports modern Mapbox/MapLibre style filters, PMTiles output, and a `vt-compat` mode that mirrors vt-optimizer layer visibility behavior.

## Features

- Inspect MBTiles/PMTiles with histograms, layer stats, and summaries
- Optimize (prune) tiles using style visibility and filters
- PMTiles input/output for optimize
- MBTiles `map/images` schema support
- `vt-compat` mode for vt-optimizer parity (filter ignored)

## Install

Build from source:

```bash
cargo build --release
```

## Usage

### Inspect

```bash
# basic summary
cargo run --quiet -- inspect /path/to/tiles.mbtiles

# PMTiles
cargo run --quiet -- inspect /path/to/tiles.pmtiles

# NDJSON output
cargo run --quiet -- inspect /path/to/tiles.mbtiles --output ndjson
```

### Optimize (prune)

```bash
# style-based pruning (layer+filter)
cargo run --quiet -- optimize /path/to/tiles.mbtiles \
  --output /path/to/tiles.pruned.mbtiles \
  --style /path/to/style.json

# vt-optimizer compatible mode (visibility only)
cargo run --quiet -- optimize /path/to/tiles.mbtiles \
  --output /path/to/tiles.pruned.mbtiles \
  --style /path/to/style.json \
  --style-mode vt-compat

# PMTiles optimize
cargo run --quiet -- optimize /path/to/tiles.pmtiles \
  --output /path/to/tiles.pruned.pmtiles \
  --style /path/to/style.json
```

### Copy

```bash
cargo run --quiet -- copy /path/to/tiles.mbtiles --output /path/to/tiles.copy.mbtiles
```

## Style modes

- `layer+filter` (default): keeps features matching supported filter expressions
- `layer`: keeps entire layers that are visible (no filter evaluation)
- `vt-compat`: same as vt-optimizer visibility behavior (min/max zoom, layout visibility, paint non-zero), filter ignored

## Notes

- Unknown filter expressions are treated as **keep** and are reported in the optimize summary.
- MBTiles with `map/images` schema are supported for inspect/copy/optimize.
- PMTiles optimize currently rewrites the archive with preserved metadata and compression.

## Development

```bash
cargo test
```
