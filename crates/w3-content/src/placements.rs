//! `units.json`（`war3mapUnits.doo`）与 `doodads.json`（`war3map.doo`）。
//!
//! 只读取 P1 所需字段；其余上游字段（背包、技能、掉落物品集、缩放等）由 serde
//! 刻意忽略，因此上游 schema 增长不会破坏读取器。

use serde::Deserialize;

#[derive(Clone, Copy, Debug, Deserialize, PartialEq)]
pub struct Position3 {
    pub x: f64,
    pub y: f64,
    pub z: f64,
}

#[derive(Clone, Debug, Deserialize)]
pub struct UnitPlacement {
    /// FourCC 单位类型 ID，例如来自 `war3mapUnits.doo`。
    #[serde(rename = "typeId")]
    pub type_id: String,
    #[serde(default)]
    pub variation: i64,
    pub position: Position3,
    #[serde(default)]
    pub angle: f64,
    #[serde(default)]
    pub owner: i64,
    #[serde(default)]
    pub flags: i64,
    #[serde(default, rename = "creationNumber")]
    pub creation_number: i64,
}

#[derive(Clone, Debug, Deserialize)]
pub struct DoodadPlacement {
    /// FourCC 装饰物 ID。
    pub id: String,
    #[serde(default)]
    pub variation: i64,
    pub position: Position3,
    #[serde(default)]
    pub angle: f64,
    #[serde(default)]
    pub flags: i64,
    #[serde(default)]
    pub life: i64,
    #[serde(default, rename = "creationNumber")]
    pub creation_number: i64,
}

#[derive(Clone, Debug, Deserialize)]
pub struct UnitsFile {
    #[serde(default, rename = "formatVersion")]
    pub format_version: i64,
    #[serde(default)]
    pub subversion: i64,
    #[serde(default)]
    pub count: usize,
    #[serde(default)]
    pub units: Vec<UnitPlacement>,
}

#[derive(Clone, Debug, Deserialize)]
pub struct DoodadsFile {
    #[serde(default, rename = "formatVersion")]
    pub format_version: i64,
    #[serde(default)]
    pub subversion: i64,
    #[serde(default)]
    pub count: usize,
    #[serde(default)]
    pub doodads: Vec<DoodadPlacement>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PlacementError {
    CountMismatch {
        file: &'static str,
        declared: usize,
        found: usize,
    },
}

impl UnitsFile {
    pub fn validate(&self) -> Result<(), PlacementError> {
        if self.count != 0 && self.count != self.units.len() {
            return Err(PlacementError::CountMismatch {
                file: "units.json",
                declared: self.count,
                found: self.units.len(),
            });
        }
        Ok(())
    }
}

impl DoodadsFile {
    pub fn validate(&self) -> Result<(), PlacementError> {
        if self.count != 0 && self.count != self.doodads.len() {
            return Err(PlacementError::CountMismatch {
                file: "doodads.json",
                declared: self.count,
                found: self.doodads.len(),
            });
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn unknown_upstream_fields_are_ignored() {
        let json = r#"{
            "formatVersion": 8, "subversion": 11, "count": 1,
            "byTypeId": {"AAAA": 1},
            "units": [{
                "typeId": "AAAA", "variation": 0,
                "position": {"x": 1.5, "y": 2.0, "z": 0.0},
                "angle": 0.0, "angleDegrees": 0.0,
                "scale": {"x": 1, "y": 1, "z": 1},
                "owner": 3, "flags": 2, "creationNumber": 42,
                "inventory": [], "abilities": [], "random": {"flag": -1}
            }],
            "_bytesRemaining": 0
        }"#;
        let file: UnitsFile = serde_json::from_str(json).unwrap();
        file.validate().unwrap();
        assert_eq!(file.units[0].type_id, "AAAA");
        assert_eq!(file.units[0].owner, 3);
        assert_eq!(file.units[0].creation_number, 42);
    }

    #[test]
    fn declared_count_must_match() {
        let json = r#"{"count": 3, "units": []}"#;
        let file: UnitsFile = serde_json::from_str(json).unwrap();
        assert_eq!(
            file.validate(),
            Err(PlacementError::CountMismatch {
                file: "units.json",
                declared: 3,
                found: 0,
            })
        );
    }
}
