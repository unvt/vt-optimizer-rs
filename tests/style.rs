use std::fs;

use tile_prune::style::read_style;

#[test]
fn style_visibility_checks_zoom_and_paint() {
    let dir = tempfile::tempdir().expect("tempdir");
    let style_path = dir.path().join("style.json");
    fs::write(
        &style_path,
        r#"{
  "version": 8,
  "sources": { "osm": { "type": "vector" } },
  "layers": [
    { "id": "water", "type": "fill", "source": "osm", "source-layer": "water", "minzoom": 2, "maxzoom": 5, "paint": { "fill-opacity": 1 } },
    { "id": "water-hidden", "type": "fill", "source": "osm", "source-layer": "water", "layout": { "visibility": "none" } },
    { "id": "roads-width", "type": "line", "source": "osm", "source-layer": "roads", "paint": { "line-width": 0 } },
    { "id": "roads-stops", "type": "line", "source": "osm", "source-layer": "roads", "paint": { "line-width": { "base": 1, "stops": [[3, 0], [4, 2]] } } }
  ]
}"#,
    )
    .expect("write style");

    let style = read_style(&style_path).expect("read style");
    let source_layers = style.source_layers();
    assert!(source_layers.contains("water"));
    assert!(source_layers.contains("roads"));

    assert!(!style.is_layer_visible_on_zoom("water", 1));
    assert!(style.is_layer_visible_on_zoom("water", 3));
    assert!(!style.is_layer_visible_on_zoom("roads", 3));
    assert!(style.is_layer_visible_on_zoom("roads", 4));
}

#[test]
fn style_filter_allows_type_checks() {
    let dir = tempfile::tempdir().expect("tempdir");
    let style_path = dir.path().join("style.json");
    fs::write(
        &style_path,
        r#"{
  "version": 8,
  "sources": { "osm": { "type": "vector" } },
  "layers": [
    { "id": "water", "type": "fill", "source": "osm", "source-layer": "water", "filter": ["==", "$type", "Polygon"] }
  ]
}"#,
    )
    .expect("write style");

    let style = read_style(&style_path).expect("read style");
    assert!(style.is_layer_visible_on_zoom("water", 0));
}

#[test]
fn style_filter_supports_zoom_reference() {
    let dir = tempfile::tempdir().expect("tempdir");
    let style_path = dir.path().join("style.json");
    fs::write(
        &style_path,
        r#"{
  "version": 8,
  "sources": { "osm": { "type": "vector" } },
  "layers": [
    { "id": "roads", "type": "line", "source": "osm", "source-layer": "roads", "filter": ["==", "zoom", 3] }
  ]
}"#,
    )
    .expect("write style");

    let style = read_style(&style_path).expect("read style");
    assert!(style.is_layer_visible_on_zoom("roads", 3));
    assert!(style.is_layer_visible_on_zoom("roads", 2));

    let feature = mvt_reader::feature::Feature {
        geometry: geo_types::Geometry::Point(geo_types::Point::new(0.0, 0.0)),
        id: None,
        properties: None,
    };
    let mut unknown = 0usize;
    assert_eq!(
        style.should_keep_feature("roads", 3, &feature, &mut unknown),
        tile_prune::style::FilterResult::True
    );
    assert_eq!(
        style.should_keep_feature("roads", 2, &feature, &mut unknown),
        tile_prune::style::FilterResult::False
    );
}

#[test]
fn style_filter_supports_get_and_in_expressions() {
    let dir = tempfile::tempdir().expect("tempdir");
    let style_path = dir.path().join("style.json");
    fs::write(
        &style_path,
        r#"{
  "version": 8,
  "sources": { "osm": { "type": "vector" } },
  "layers": [
    { "id": "roads", "type": "line", "source": "osm", "source-layer": "roads",
      "filter": ["any",
        ["==", ["get", "class"], "primary"],
        ["in", ["get", "class"], "secondary", "tertiary"]
      ]
    }
  ]
}"#,
    )
    .expect("write style");

    let style = read_style(&style_path).expect("read style");
    let feature_primary = mvt_reader::feature::Feature {
        geometry: geo_types::Geometry::Point(geo_types::Point::new(0.0, 0.0)),
        id: None,
        properties: Some(
            [(
                "class".to_string(),
                mvt_reader::feature::Value::String("primary".to_string()),
            )]
            .into_iter()
            .collect(),
        ),
    };
    let feature_secondary = mvt_reader::feature::Feature {
        geometry: geo_types::Geometry::Point(geo_types::Point::new(0.0, 0.0)),
        id: None,
        properties: Some(
            [(
                "class".to_string(),
                mvt_reader::feature::Value::String("secondary".to_string()),
            )]
            .into_iter()
            .collect(),
        ),
    };
    let feature_other = mvt_reader::feature::Feature {
        geometry: geo_types::Geometry::Point(geo_types::Point::new(0.0, 0.0)),
        id: None,
        properties: Some(
            [(
                "class".to_string(),
                mvt_reader::feature::Value::String("service".to_string()),
            )]
            .into_iter()
            .collect(),
        ),
    };
    let mut unknown = 0usize;
    assert_eq!(
        style.should_keep_feature("roads", 3, &feature_primary, &mut unknown),
        tile_prune::style::FilterResult::True
    );
    assert_eq!(
        style.should_keep_feature("roads", 3, &feature_secondary, &mut unknown),
        tile_prune::style::FilterResult::True
    );
    assert_eq!(
        style.should_keep_feature("roads", 3, &feature_other, &mut unknown),
        tile_prune::style::FilterResult::False
    );
}

#[test]
fn style_filter_supports_match_case_coalesce() {
    let dir = tempfile::tempdir().expect("tempdir");
    let style_path = dir.path().join("style.json");
    fs::write(
        &style_path,
        r#"{
  "version": 8,
  "sources": { "osm": { "type": "vector" } },
  "layers": [
    { "id": "roads", "type": "line", "source": "osm", "source-layer": "roads",
      "filter": ["all",
        ["==", ["match", ["get", "class"], "primary", "keep", "secondary", "drop", "default"], "keep"],
        ["==", ["case", ["==", ["get", "class"], "primary"], "yes", ["==", ["get", "class"], "secondary"], "yes", "no"], "yes"],
        ["==", ["coalesce", ["get", "name"], ["get", "class"], "fallback"], "primary"]
      ]
    }
  ]
}"#,
    )
    .expect("write style");

    let style = read_style(&style_path).expect("read style");
    let feature_primary = mvt_reader::feature::Feature {
        geometry: geo_types::Geometry::Point(geo_types::Point::new(0.0, 0.0)),
        id: None,
        properties: Some(
            [(
                "class".to_string(),
                mvt_reader::feature::Value::String("primary".to_string()),
            )]
            .into_iter()
            .collect(),
        ),
    };
    let feature_secondary = mvt_reader::feature::Feature {
        geometry: geo_types::Geometry::Point(geo_types::Point::new(0.0, 0.0)),
        id: None,
        properties: Some(
            [(
                "class".to_string(),
                mvt_reader::feature::Value::String("secondary".to_string()),
            )]
            .into_iter()
            .collect(),
        ),
    };
    let feature_missing = mvt_reader::feature::Feature {
        geometry: geo_types::Geometry::Point(geo_types::Point::new(0.0, 0.0)),
        id: None,
        properties: None,
    };
    let mut unknown = 0usize;
    assert_eq!(
        style.should_keep_feature("roads", 3, &feature_primary, &mut unknown),
        tile_prune::style::FilterResult::True
    );
    assert_eq!(
        style.should_keep_feature("roads", 3, &feature_secondary, &mut unknown),
        tile_prune::style::FilterResult::False
    );
    assert_eq!(
        style.should_keep_feature("roads", 3, &feature_missing, &mut unknown),
        tile_prune::style::FilterResult::False
    );
}
