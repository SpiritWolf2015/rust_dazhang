//! `tools/slk-export` 产出的 `slk-exported/*.json`（每张 SLK 表一个 JSON）。
//!
//! 载荷结构：`{ source, columns, rows, headers, recordCount, records }`。
//! `records` 是以列名为键的行对象列表。

use std::collections::BTreeMap;

use serde::Deserialize;
use serde_json::Value;

#[derive(Clone, Debug, Deserialize)]
pub struct SlkTable {
    #[serde(default)]
    pub source: Option<String>,
    #[serde(default)]
    pub columns: usize,
    #[serde(default)]
    pub rows: usize,
    #[serde(default)]
    pub headers: Vec<String>,
    #[serde(default, rename = "recordCount")]
    pub record_count: usize,
    #[serde(default)]
    pub records: Vec<BTreeMap<String, Value>>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TableError {
    RecordCountMismatch { declared: usize, found: usize },
}

impl SlkTable {
    pub fn validate(&self) -> Result<(), TableError> {
        if self.record_count != 0 && self.record_count != self.records.len() {
            return Err(TableError::RecordCountMismatch {
                declared: self.record_count,
                found: self.records.len(),
            });
        }
        Ok(())
    }

    /// 返回 `column` 列的字符串等于 `value` 的第一行。
    /// BTreeMap 保证行内字段顺序稳定，与 JSON 键顺序无关。
    pub fn find_row(&self, column: &str, value: &str) -> Option<&BTreeMap<String, Value>> {
        self.records
            .iter()
            .find(|row| row.get(column).and_then(Value::as_str) == Some(value))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn find_row_is_stable_and_typed() {
        let json = r#"{
            "source": "Units/Synthetic.slk",
            "columns": 3, "rows": 2,
            "headers": ["unitID", "Name", "hitPoints"],
            "recordCount": 2,
            "records": [
                {"hitPoints": 100, "unitID": "AAAA", "Name": "Synthetic A"},
                {"unitID": "BBBB", "Name": "Synthetic B", "hitPoints": 200}
            ]
        }"#;
        let table: SlkTable = serde_json::from_str(json).unwrap();
        table.validate().unwrap();
        let row = table.find_row("unitID", "BBBB").unwrap();
        assert_eq!(row.get("hitPoints").and_then(Value::as_i64), Some(200));
    }

    #[test]
    fn declared_record_count_must_match() {
        let json = r#"{"recordCount": 5, "records": []}"#;
        let table: SlkTable = serde_json::from_str(json).unwrap();
        assert_eq!(
            table.validate(),
            Err(TableError::RecordCountMismatch {
                declared: 5,
                found: 0,
            })
        );
    }
}
