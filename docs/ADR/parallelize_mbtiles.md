# ADR: parallelize_mbtiles

Date: 2026-01-20
Status: Accepted

## Context

Large MBTiles sets can exceed 100M tiles. A single-threaded scan of tile sizes
(`LENGTH(tile_data)`) becomes a dominant cost for `inspect` histogram and
zoom histogram steps. Some MBTiles variants use a shallow schema:

- `tiles_shallow` WITHOUT ROWID, primary key `(zoom_level, tile_column, tile_row)`
- `tiles_data` with `tile_data_id` (INTEGER PRIMARY KEY) and `tile_data` (BLOB)
- `tiles` view joins `tiles_shallow` and `tiles_data`

Because `tiles_shallow` is WITHOUT ROWID, parallel scans cannot be split by
rowid ranges. The safe key to split work is `zoom_level`, which is indexed as
part of the primary key.

## Decision

Parallelize MBTiles scans by zoom level using multiple read-only SQLite
connections. Each worker executes:

- `SELECT LENGTH(tile_data) FROM <source> WHERE zoom_level = ?`

Workers aggregate per-zoom counts/bytes in memory and reduce the results into
final histogram structures. This uses Rayon for CPU parallelism while allowing
SQLite to perform I/O from multiple readers.

For `inspect` processing, use a two-pass strategy:

1) Pass 1: per-zoom scan to compute stats, min/max, topn, and sampled metadata
2) Pass 2: per-zoom scan to compute bucket/list results using the finalized
   min/max bounds

This removes order-dependence from bucket selection while preserving accuracy.

Sampling is applied per zoom level:

- `SampleSpec::Count(n)` means up to `n` tiles per zoom
- `SampleSpec::Ratio(r)` applies the ratio within each zoom

## Consequences

- Histogram steps use multiple SQLite connections (read-only).
- CPU utilization increases during histogram building on multi-core systems.
- Memory use stays bounded by per-zoom bucket accumulators (no full tile list).
- Sampling semantics change to be zoom-scoped for inspect results.

## Notes

This approach avoids rowid-based chunking and works with both `tiles` tables
and `map/images` schemas because the zoom column is present in both.
