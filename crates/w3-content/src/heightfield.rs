//! `terrain-heightfield.json` from `tools/map-parse` (parallel vertex arrays).
//!
//! Header fields follow `docs/architecture/MAP_DATA.md`: tilepoint dimensions
//! are `mapWidth + 1` / `mapHeight + 1`, and every parallel array has
//! `tilepointWidth * tilepointHeight` entries.

use serde::Deserialize;

#[derive(Clone, Copy, Debug, Deserialize, PartialEq)]
pub struct CenterOffset {
    pub x: f64,
    pub y: f64,
}

#[derive(Clone, Debug, Deserialize)]
pub struct TerrainHeightfield {
    #[serde(rename = "tilepointWidth")]
    pub tilepoint_width: usize,
    #[serde(rename = "tilepointHeight")]
    pub tilepoint_height: usize,
    #[serde(rename = "mapWidth")]
    pub map_width: usize,
    #[serde(rename = "mapHeight")]
    pub map_height: usize,

    #[serde(rename = "centerOffset", default)]
    pub center_offset: Option<CenterOffset>,
    #[serde(rename = "mainTileset", default)]
    pub main_tileset: Option<String>,
    #[serde(rename = "mainTilesetName", default)]
    pub main_tileset_name: Option<String>,
    #[serde(rename = "groundTilesets", default)]
    pub ground_tilesets: Vec<String>,
    #[serde(rename = "cliffTilesets", default)]
    pub cliff_tilesets: Vec<String>,
    #[serde(rename = "tileSize", default)]
    pub tile_size: Option<f64>,

    /// Final ground height per tilepoint. Required.
    pub heights: Vec<f64>,

    // Optional parallel arrays: absent in some exports, but when present they
    // must have exactly vertex_count() entries.
    #[serde(rename = "groundTextures", default)]
    pub ground_textures: Vec<i64>,
    #[serde(rename = "groundVariations", default)]
    pub ground_variations: Vec<i64>,
    #[serde(rename = "cliffTextures", default)]
    pub cliff_textures: Vec<i64>,
    #[serde(rename = "cliffVariations", default)]
    pub cliff_variations: Vec<i64>,
    #[serde(rename = "layerHeights", default)]
    pub layer_heights: Vec<i64>,
    #[serde(rename = "waterHeights", default)]
    pub water_heights: Vec<f64>,
    #[serde(rename = "flagsPacked", default)]
    pub flags_packed: Vec<i64>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum HeightfieldError {
    DimensionOverflow,
    WrongArrayLength {
        field: &'static str,
        found: usize,
        expected: usize,
    },
    MapSizeMismatch {
        map_width: usize,
        tilepoint_width: usize,
        map_height: usize,
        tilepoint_height: usize,
    },
}

impl TerrainHeightfield {
    pub fn vertex_count(&self) -> Option<usize> {
        self.tilepoint_width.checked_mul(self.tilepoint_height)
    }

    /// Validate width/height consistency and every present parallel array.
    pub fn validate(&self) -> Result<usize, HeightfieldError> {
        let expected = self
            .vertex_count()
            .ok_or(HeightfieldError::DimensionOverflow)?;
        if self.heights.len() != expected {
            return Err(HeightfieldError::WrongArrayLength {
                field: "heights",
                found: self.heights.len(),
                expected,
            });
        }

        let optional: [(&'static str, usize); 7] = [
            ("groundTextures", self.ground_textures.len()),
            ("groundVariations", self.ground_variations.len()),
            ("cliffTextures", self.cliff_textures.len()),
            ("cliffVariations", self.cliff_variations.len()),
            ("layerHeights", self.layer_heights.len()),
            ("waterHeights", self.water_heights.len()),
            ("flagsPacked", self.flags_packed.len()),
        ];
        for (field, found) in optional {
            if found != 0 && found != expected {
                return Err(HeightfieldError::WrongArrayLength {
                    field,
                    found,
                    expected,
                });
            }
        }

        let width_ok =
            self.map_width == 0 || self.map_width.checked_add(1) == Some(self.tilepoint_width);
        let height_ok =
            self.map_height == 0 || self.map_height.checked_add(1) == Some(self.tilepoint_height);
        if !width_ok || !height_ok {
            return Err(HeightfieldError::MapSizeMismatch {
                map_width: self.map_width,
                tilepoint_width: self.tilepoint_width,
                map_height: self.map_height,
                tilepoint_height: self.tilepoint_height,
            });
        }

        Ok(expected)
    }

    /// `index = iy * tilepointWidth + ix`, matching MAP_DATA.md.
    pub fn height_at(&self, ix: usize, iy: usize) -> Option<f64> {
        // Without this, ix past the row width would wrap into the next row.
        if ix >= self.tilepoint_width || iy >= self.tilepoint_height {
            return None;
        }
        let index = iy.checked_mul(self.tilepoint_width)?.checked_add(ix)?;
        self.heights.get(index).copied()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const VALID: &str = r#"{
        "tilepointWidth": 3, "tilepointHeight": 3,
        "mapWidth": 2, "mapHeight": 2,
        "heights": [0,1,2,3,4,5,6,7,8],
        "groundTextures": [0,1,0,1,0,1,0,1,0],
        "flagsPacked": [0,0,0,0,0,0,0,0,0]
    }"#;

    #[test]
    fn valid_heightfield_passes_and_indexes_vertices() {
        let hf: TerrainHeightfield = serde_json::from_str(VALID).unwrap();
        assert_eq!(hf.validate(), Ok(9));
        assert_eq!(hf.height_at(0, 0), Some(0.0));
        assert_eq!(hf.height_at(2, 1), Some(5.0));
        assert_eq!(hf.height_at(3, 0), None);
    }

    #[test]
    fn short_required_array_is_rejected() {
        let json = VALID.replace("[0,1,2,3,4,5,6,7,8]", "[0,1,2]");
        let hf: TerrainHeightfield = serde_json::from_str(&json).unwrap();
        assert_eq!(
            hf.validate(),
            Err(HeightfieldError::WrongArrayLength {
                field: "heights",
                found: 3,
                expected: 9,
            })
        );
    }

    #[test]
    fn short_optional_array_is_rejected() {
        let json = VALID.replace("[0,1,0,1,0,1,0,1,0]", "[0,1]");
        let hf: TerrainHeightfield = serde_json::from_str(&json).unwrap();
        assert_eq!(
            hf.validate(),
            Err(HeightfieldError::WrongArrayLength {
                field: "groundTextures",
                found: 2,
                expected: 9,
            })
        );
    }

    #[test]
    fn map_size_must_match_tilepoint_dimensions() {
        let json = VALID.replace("\"mapWidth\": 2", "\"mapWidth\": 5");
        let hf: TerrainHeightfield = serde_json::from_str(&json).unwrap();
        assert!(matches!(
            hf.validate(),
            Err(HeightfieldError::MapSizeMismatch { .. })
        ));
    }

    #[test]
    fn absent_optional_arrays_are_allowed() {
        let json = r#"{
            "tilepointWidth": 2, "tilepointHeight": 2,
            "mapWidth": 1, "mapHeight": 1,
            "heights": [0,0,0,0]
        }"#;
        let hf: TerrainHeightfield = serde_json::from_str(json).unwrap();
        assert_eq!(hf.validate(), Ok(4));
    }
}
