# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project Overview

**tile-prune** is a Rust CLI tool and SDK for inspecting, optimizing, and pruning Mapbox Vector Tiles (MVT) stored in MBTiles or PMTiles formats. It targets planet-scale tilesets (e.g., 92GB+ files) with streaming processing, parallel reads, and single-writer architecture.

Key capabilities:
- **inspect**: Analyze tile statistics, size distributions, and layer metadata
- **optimize**: Prune unused layers and features based on Mapbox/MapLibre style specifications
- **simplify**: Reduce geometry vertices to decrease tile size
- **copy**: Convert between MBTiles and PMTiles formats

## Building and Testing

```bash
# Build the project
cargo build

# Build release version
cargo build --release

# Run tests
cargo test

# Run specific test
cargo test test_name

# Run with logging
RUST_LOG=debug cargo run -- inspect input.mbtiles
```

## Running Commands

```bash
# Inspect an MBTiles file
cargo run -- inspect input.mbtiles

# Inspect with custom options
cargo run -- inspect input.mbtiles --histogram-buckets 10 --topn 5 --max-tile-bytes 1280000

# Inspect specific zoom level
cargo run -- inspect input.mbtiles --zoom 10

# Inspect with JSON output
cargo run -- inspect input.mbtiles --output json

# Inspect specific tile with summary
cargo run -- inspect input.mbtiles --tile 10/512/384 --summary

# Copy MBTiles to PMTiles
cargo run -- copy input.mbtiles --output output.pmtiles

# Copy PMTiles to MBTiles
cargo run -- copy input.pmtiles --output output.mbtiles

# Optimize (currently skeleton only)
cargo run -- optimize input.mbtiles --style style.json --output output.mbtiles
```

## Architecture

### Core Design Principles

1. **Streaming Processing**: Never load entire tilesets into memory; process tiles in streams
2. **Parallel Reads + Single Writer**: Multiple reader threads with one aggregating writer thread
3. **SQLite WAL Mode**: Leverages Write-Ahead Logging for concurrent reader/writer operations
4. **Conservative Pruning**: When uncertain (e.g., unsupported filter expressions), preserve data rather than risk removing needed content

### Project Structure

```
src/
├── lib.rs           # Module exports
├── main.rs          # CLI entry point with command routing
├── cli.rs           # Command-line argument definitions (clap)
├── format.rs        # Format detection and conversion planning
├── mbtiles.rs       # MBTiles reader/writer core logic
├── pmtiles.rs       # PMTiles reader/writer core logic
└── output.rs        # Report formatting (text/json/ndjson)

tests/               # Integration tests
```

### Key Modules

- **cli.rs**: Defines CLI structure using `clap` with subcommands (inspect/optimize/simplify/copy/verify)
- **format.rs**: Handles format detection from file extensions and plans input/output conversions
- **mbtiles.rs**: Core SQLite operations for MBTiles including inspection with statistical analysis
- **pmtiles.rs**: PMTiles v3 read/write operations
- **output.rs**: Formats inspection reports in text, JSON, or NDJSON

### Coordinate Systems

- **Internal representation**: XYZ (z/x/y) throughout the codebase
- **MBTiles storage**: Uses TMS (Tile Map Service) with Y-axis inversion
- **Conversion formula**: `y_tms = (2^z - 1) - y_xyz`
- Always convert at I/O boundaries; never mix coordinate systems internally

### Tile Format Details

**MBTiles**: SQLite container with `tiles(zoom_level, tile_column, tile_row, tile_data)` table. May use normalized schema with views.

**PMTiles v3**: Single-file archive format designed for HTTP Range requests, but used here as local files. Read-only format requiring full regeneration (no in-place updates).

**MVT (Mapbox Vector Tile)**: Protocol Buffers format with extent=4096 (de facto standard), usually gzip-compressed.

### Style Interpretation (Future Implementation)

When implementing the `optimize` command's style-based pruning:

1. **Filter semantics**: Filters define "matching features to display"; non-matching features are prune candidates
2. **Zoom evaluation**: Evaluate zoom expressions at integer zoom levels only
3. **Feature-state**: Not supported in filters (per Mapbox/MapLibre spec)
4. **Unknown expressions**: Conservative "keep" behavior - preserve data when encountering unsupported filter syntax
5. **Multi-layer OR logic**: When multiple style layers reference the same source-layer, keep features matching ANY layer's filter (logical OR)

Style modes (via `--style-mode`):
- `none`: No pruning
- `layer`: Remove unused layers only
- `layer+filter`: Remove unused layers AND filter features (default)

## Development Milestones

The project follows incremental milestones documented in SPEC.md:
- **v0.0.1-v0.0.2**: CLI skeleton and basic MBTiles copy/inspect
- **v0.0.3**: Cross-format copy (MBTiles ↔ PMTiles)
- **v0.0.4-v0.0.30**: Progressive enhancements to inspect command (sampling, histograms, layer analysis, progress indicators)
- **Future**: Style-based optimization, simplification, checkpoint/resume

## Important Implementation Notes

### MBTiles Operations

- Use `rusqlite` with bundled SQLite (no external dependency)
- Enable WAL mode for concurrent reads: `PRAGMA journal_mode=WAL`
- Use transactions for batch writes to improve performance
- Handle both standard and normalized schemas transparently via `tiles` view

### PMTiles Operations

- Currently supports v3 format only
- Use `pmtiles` crate for reading/writing
- PMTiles requires full regeneration; cannot update in-place

### MVT Decoding

- Use `mvt-reader` crate for parsing Protocol Buffers
- Decompress with `flate2` (gzip)
- Handle decompression failures gracefully (try raw PBF if gzip fails)
- Parse layer metadata: name, feature count, property keys, extent, version

### Error Handling Philosophy

- **Decode failures**: Warn and keep original tile data by default
- **Unknown filters**: Conservative "keep" to avoid over-pruning
- **Invalid input**: Validate early and provide clear error messages
- Use `anyhow` for error propagation with context

### Performance Considerations

- **Target**: Process 92GB planet.mbtiles in ~6 hours on 32 vCPU / 96GB RAM / NVMe
- Batch SQLite operations (default: 1000 tiles per transaction)
- Use bounded channels for backpressure in pipeline
- Sample mode (`--sample`) for faster analysis of large tilesets
- Progress indicators with `indicatif` crate

## Testing Strategy

- Unit tests for format detection, coordinate conversion, option parsing
- Integration tests for full command workflows
- Test with both MBTiles and PMTiles inputs
- Validate output correctness and format conversion accuracy

## References

- **PRD.md**: Product requirements and goals
- **ARD.md**: Architectural decisions with rationale
- **SPEC.md**: Detailed technical specification with milestone breakdown
- **Mapbox Vector Tile Spec**: https://github.com/mapbox/vector-tile-spec
- **MBTiles 1.3 Spec**: https://github.com/mapbox/mbtiles-spec
- **PMTiles v3 Spec**: https://github.com/protomaps/PMTiles
