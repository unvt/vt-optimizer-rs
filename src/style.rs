use std::collections::{HashMap, HashSet};
use std::fs;
use std::path::Path;

use anyhow::{Context, Result};
use serde_json::Value;

const PAINT_PROPERTIES_TO_CHECK: &[&str] = &[
    "fill-opacity",
    "fill-outline-color",
    "line-opacity",
    "line-width",
    "icon-size",
    "text-size",
    "text-max-width",
    "text-opacity",
    "raster-opacity",
    "circle-radius",
    "circle-opacity",
    "fill-extrusion-opacity",
    "heatmap-opacity",
];

#[derive(Debug, Clone)]
enum PaintValue {
    Number(f64),
    Stops(Vec<(u8, f64)>),
}

impl PaintValue {
    fn is_nonzero_at_zoom(&self, zoom: u8) -> bool {
        match self {
            PaintValue::Number(value) => *value != 0.0,
            PaintValue::Stops(stops) => {
                if let Some((_, value)) = stops.iter().find(|(z, _)| *z == zoom) {
                    *value != 0.0
                } else {
                    true
                }
            }
        }
    }
}

#[derive(Debug, Clone)]
struct MapboxStyleLayer {
    minzoom: Option<f64>,
    maxzoom: Option<f64>,
    visibility: Option<String>,
    paint: HashMap<String, PaintValue>,
}

impl MapboxStyleLayer {
    fn is_visible_on_zoom(&self, zoom: u8) -> bool {
        self.check_layout_visibility() && self.check_zoom_underflow(zoom) && self.check_zoom_overflow(zoom)
    }

    fn check_layout_visibility(&self) -> bool {
        match self.visibility.as_deref() {
            Some("none") => false,
            _ => true,
        }
    }

    fn check_zoom_underflow(&self, zoom: u8) -> bool {
        self.minzoom.map_or(true, |minzoom| (zoom as f64) >= minzoom)
    }

    fn check_zoom_overflow(&self, zoom: u8) -> bool {
        self.maxzoom.map_or(true, |maxzoom| maxzoom > (zoom as f64))
    }

    fn is_rendered(&self, zoom: u8) -> bool {
        for prop in PAINT_PROPERTIES_TO_CHECK {
            if !self.check_paint_property_not_zero(prop, zoom) {
                return false;
            }
        }
        true
    }

    fn check_paint_property_not_zero(&self, property: &str, zoom: u8) -> bool {
        match self.paint.get(property) {
            Some(value) => value.is_nonzero_at_zoom(zoom),
            None => true,
        }
    }
}

#[derive(Debug, Clone)]
pub struct MapboxStyle {
    layers_by_source_layer: HashMap<String, Vec<MapboxStyleLayer>>,
}

impl MapboxStyle {
    pub fn source_layers(&self) -> HashSet<String> {
        self.layers_by_source_layer.keys().cloned().collect()
    }

    pub fn is_layer_visible_on_zoom(&self, layer_name: &str, zoom: u8) -> bool {
        self.layers_by_source_layer
            .get(layer_name)
            .map(|layers| {
                layers.iter().any(|layer| {
                    layer.is_visible_on_zoom(zoom) && layer.is_rendered(zoom)
                })
            })
            .unwrap_or(false)
    }
}

fn parse_paint_value(value: &Value) -> Option<PaintValue> {
    if let Some(number) = value.as_f64() {
        return Some(PaintValue::Number(number));
    }
    let stops = value.get("stops")?.as_array()?;
    let mut parsed = Vec::new();
    for stop in stops {
        let arr = stop.as_array()?;
        if arr.len() < 2 {
            continue;
        }
        let zoom = arr[0].as_f64()? as i64;
        let value = arr[1].as_f64()?;
        if !(0..=255).contains(&zoom) {
            continue;
        }
        parsed.push((zoom as u8, value));
    }
    if parsed.is_empty() {
        None
    } else {
        Some(PaintValue::Stops(parsed))
    }
}

pub fn read_style(path: &Path) -> Result<MapboxStyle> {
    let contents = fs::read_to_string(path)
        .with_context(|| format!("failed to read style file: {}", path.display()))?;
    let value: Value = serde_json::from_str(&contents).context("parse style json")?;
    let layers = value
        .get("layers")
        .and_then(|layers| layers.as_array())
        .ok_or_else(|| anyhow::anyhow!("style json missing layers array"))?;

    let mut layers_by_source_layer: HashMap<String, Vec<MapboxStyleLayer>> = HashMap::new();
    for layer in layers {
        if layer.get("source").is_none() {
            continue;
        }
        let Some(source_layer) = layer.get("source-layer").and_then(|v| v.as_str()) else {
            continue;
        };
        let minzoom = layer.get("minzoom").and_then(|v| v.as_f64());
        let maxzoom = layer.get("maxzoom").and_then(|v| v.as_f64());
        let visibility = layer
            .get("layout")
            .and_then(|layout| layout.get("visibility"))
            .and_then(|v| v.as_str())
            .map(|v| v.to_string());
        let mut paint = HashMap::new();
        if let Some(props) = layer.get("paint").and_then(|paint| paint.as_object()) {
            for (key, value) in props {
                if let Some(parsed) = parse_paint_value(value) {
                    paint.insert(key.clone(), parsed);
                }
            }
        }
        layers_by_source_layer
            .entry(source_layer.to_string())
            .or_default()
            .push(MapboxStyleLayer {
                minzoom,
                maxzoom,
                visibility,
                paint,
            });
    }

    if layers_by_source_layer.is_empty() {
        anyhow::bail!("style json contains no source-layer entries");
    }
    Ok(MapboxStyle {
        layers_by_source_layer,
    })
}

pub fn read_style_source_layers(path: &Path) -> Result<HashSet<String>> {
    Ok(read_style(path)?.source_layers())
}
