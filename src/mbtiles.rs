use std::path::Path;

use anyhow::{Context, Result};
use indicatif::{ProgressBar, ProgressStyle};
use rusqlite::{params, Connection, OpenFlags};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MbtilesStats {
    pub tile_count: u64,
    pub total_bytes: u64,
    pub max_bytes: u64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MbtilesZoomStats {
    pub zoom: u8,
    pub stats: MbtilesStats,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MbtilesReport {
    pub overall: MbtilesStats,
    pub by_zoom: Vec<MbtilesZoomStats>,
}

fn ensure_mbtiles_path(path: &Path) -> Result<()> {
    let ext = path.extension().and_then(|ext| ext.to_str()).unwrap_or("");
    if ext.eq_ignore_ascii_case("mbtiles") {
        Ok(())
    } else {
        anyhow::bail!("only .mbtiles paths are supported in v0.0.3");
    }
}

fn open_readonly_mbtiles(path: &Path) -> Result<Connection> {
    Connection::open_with_flags(path, OpenFlags::SQLITE_OPEN_READ_ONLY)
        .with_context(|| format!("failed to open mbtiles: {}", path.display()))
}

fn apply_read_pragmas(conn: &Connection) -> Result<()> {
    conn.execute_batch(
        "
        PRAGMA query_only = ON;
        PRAGMA temp_store = MEMORY;
        PRAGMA synchronous = OFF;
        PRAGMA cache_size = -200000;
        ",
    )
    .context("failed to apply read pragmas")?;
    Ok(())
}

fn make_progress_bar(total: u64) -> ProgressBar {
    let bar = ProgressBar::new(total);
    bar.set_style(
        ProgressStyle::with_template("[{elapsed_precise}] {bar:40.cyan/blue} {pos}/{len} {msg}")
            .unwrap()
            .progress_chars("=>-"),
    );
    bar
}

pub fn inspect_mbtiles(path: &Path) -> Result<MbtilesReport> {
    ensure_mbtiles_path(path)?;
    let conn = open_readonly_mbtiles(path)?;
    apply_read_pragmas(&conn)?;

    let total_tiles: u64 = conn
        .query_row("SELECT COUNT(*) FROM tiles", [], |row| row.get(0))
        .context("failed to read tile count")?;
    let progress = make_progress_bar(total_tiles);

    let mut overall = MbtilesStats {
        tile_count: 0,
        total_bytes: 0,
        max_bytes: 0,
    };

    let mut stmt = conn
        .prepare("SELECT zoom_level, LENGTH(tile_data) FROM tiles ORDER BY zoom_level")
        .context("prepare tiles scan")?;
    let mut rows = stmt.query([]).context("query tiles scan")?;

    let mut by_zoom = Vec::<MbtilesZoomStats>::new();
    let mut current_zoom: Option<u8> = None;
    let mut current_stats = MbtilesStats {
        tile_count: 0,
        total_bytes: 0,
        max_bytes: 0,
    };

    let mut processed: u64 = 0;
    while let Some(row) = rows.next().context("read tile row")? {
        let zoom: u8 = row.get(0)?;
        let length: u64 = row.get(1)?;

        overall.tile_count += 1;
        overall.total_bytes += length;
        overall.max_bytes = overall.max_bytes.max(length);

        match current_zoom {
            Some(z) if z == zoom => {}
            Some(z) => {
                by_zoom.push(MbtilesZoomStats {
                    zoom: z,
                    stats: current_stats.clone(),
                });
                current_stats = MbtilesStats {
                    tile_count: 0,
                    total_bytes: 0,
                    max_bytes: 0,
                };
                current_zoom = Some(zoom);
            }
            None => current_zoom = Some(zoom),
        }

        current_stats.tile_count += 1;
        current_stats.total_bytes += length;
        current_stats.max_bytes = current_stats.max_bytes.max(length);

        processed += 1;
        if processed % 1000 == 0 {
            progress.set_position(processed);
        }
    }

    if let Some(z) = current_zoom {
        by_zoom.push(MbtilesZoomStats {
            zoom: z,
            stats: current_stats,
        });
    }

    progress.set_position(processed);
    progress.finish_and_clear();

    Ok(MbtilesReport { overall, by_zoom })
}

pub fn copy_mbtiles(input: &Path, output: &Path) -> Result<()> {
    ensure_mbtiles_path(input)?;
    ensure_mbtiles_path(output)?;
    let input_conn = Connection::open(input)
        .with_context(|| format!("failed to open input mbtiles: {}", input.display()))?;
    let mut output_conn = Connection::open(output)
        .with_context(|| format!("failed to open output mbtiles: {}", output.display()))?;

    output_conn
        .execute_batch(
            "
            CREATE TABLE metadata (name TEXT, value TEXT);
            CREATE TABLE tiles (
                zoom_level INTEGER,
                tile_column INTEGER,
                tile_row INTEGER,
                tile_data BLOB
            );
            ",
        )
        .context("failed to create output schema")?;

    let tx = output_conn.transaction().context("begin output transaction")?;

    {
        let mut stmt = input_conn
            .prepare("SELECT name, value FROM metadata")
            .context("prepare metadata")?;
        let mut rows = stmt.query([]).context("query metadata")?;
        while let Some(row) = rows.next().context("read metadata row")? {
            let name: String = row.get(0)?;
            let value: String = row.get(1)?;
            tx.execute(
                "INSERT INTO metadata (name, value) VALUES (?1, ?2)",
                params![name, value],
            )
            .context("insert metadata")?;
        }
    }

    {
        let mut stmt = input_conn
            .prepare(
                "SELECT zoom_level, tile_column, tile_row, tile_data FROM tiles ORDER BY zoom_level, tile_column, tile_row",
            )
            .context("prepare tiles")?;
        let mut rows = stmt.query([]).context("query tiles")?;
        while let Some(row) = rows.next().context("read tile row")? {
            let z: i64 = row.get(0)?;
            let x: i64 = row.get(1)?;
            let y: i64 = row.get(2)?;
            let data: Vec<u8> = row.get(3)?;
            tx.execute(
                "INSERT INTO tiles (zoom_level, tile_column, tile_row, tile_data) VALUES (?1, ?2, ?3, ?4)",
                params![z, x, y, data],
            )
            .context("insert tile")?;
        }
    }

    tx.commit().context("commit output")?;
    Ok(())
}
