use std::collections::HashMap;

use serde::Deserialize;
use serde_json::Value;

use super::daemon::ResilientExifToolDaemon;

/// A map of fully-qualified metadata keys (`Namespace:Key`) to their string
/// representation.  Values are `None` when ExifTool returns a null/binary blob.
pub type MetadataMap = HashMap<String, Option<String>>;

/// Parsed metadata for a single file.
pub struct ExifData {
    pub path: String,
    pub tags: MetadataMap,
    /// Original JSON value for consumers that need the raw structure.
    pub raw: Value,
}

/// Extract metadata from `path` using the provided daemon.
///
/// ExifTool is invoked with `-json -all:all -struct`; the JSON array response
/// is parsed and the first element's fields are flattened into `MetadataMap`.
pub async fn extract_metadata(
    daemon: &ResilientExifToolDaemon,
    path: &str,
) -> Result<ExifData, String> {
    let json_text = daemon.extract_json(path).await?;
    parse_exiftool_json(&json_text, path)
}

/// Parse the JSON text returned by ExifTool `-json` into an `ExifData`.
///
/// ExifTool returns a JSON array; we use the first element.  Each key is
/// already namespace-qualified (e.g. `"EXIF:Make"`).  Values that are objects
/// or arrays are serialised back to a JSON string.  Binary/blob fields
/// (detected by ExifTool emitting `"(Binary data …)"`) are stored as `None`.
pub fn parse_exiftool_json(json_text: &str, path: &str) -> Result<ExifData, String> {
    let array: Vec<Value> =
        serde_json::from_str(json_text.trim()).map_err(|e| format!("JSON parse error: {e}"))?;

    let obj = array
        .into_iter()
        .next()
        .ok_or("ExifTool returned empty array")?;

    let raw = obj.clone();

    let mut tags: MetadataMap = HashMap::new();

    if let Value::Object(map) = obj {
        for (key, val) in map {
            // Skip the synthetic SourceFile key ExifTool always emits.
            if key == "SourceFile" {
                continue;
            }
            let str_val = value_to_string(&val);
            tags.insert(key, str_val);
        }
    }

    Ok(ExifData {
        path: path.to_string(),
        tags,
        raw,
    })
}

/// Convert a `serde_json::Value` to an `Option<String>`:
/// - `null` → `None`
/// - scalar → `Some(stringified)`
/// - binary data marker → `None`
/// - array/object → `Some(json_string)`
fn value_to_string(val: &Value) -> Option<String> {
    match val {
        Value::Null => None,
        Value::Bool(b) => Some(b.to_string()),
        Value::Number(n) => Some(n.to_string()),
        Value::String(s) => {
            // ExifTool marks binary blobs like "(Binary data 1234 bytes, use -b option to extract)"
            if s.starts_with("(Binary data") {
                None
            } else {
                Some(s.clone())
            }
        }
        Value::Array(_) | Value::Object(_) => {
            serde_json::to_string(val).ok()
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_basic_exiftool_json() {
        let json = r#"[{"SourceFile":"/tmp/photo.jpg","EXIF:Make":"Canon","EXIF:Model":"EOS R5","XMP:Title":"My Photo","File:FileSize":"1234 kB"}]"#;
        let data = parse_exiftool_json(json, "/tmp/photo.jpg").unwrap();
        assert_eq!(data.path, "/tmp/photo.jpg");
        assert_eq!(data.tags.get("EXIF:Make").unwrap().as_deref(), Some("Canon"));
        assert_eq!(data.tags.get("XMP:Title").unwrap().as_deref(), Some("My Photo"));
        assert!(!data.tags.contains_key("SourceFile"), "SourceFile should be stripped");
    }

    #[test]
    fn handles_null_values() {
        let json = r#"[{"SourceFile":"/tmp/x.jpg","XMP:Description":null}]"#;
        let data = parse_exiftool_json(json, "/tmp/x.jpg").unwrap();
        assert_eq!(data.tags.get("XMP:Description"), Some(&None));
    }

    #[test]
    fn handles_binary_data_marker() {
        let json = r#"[{"SourceFile":"/tmp/x.jpg","EXIF:ThumbnailImage":"(Binary data 1234 bytes, use -b option to extract)"}]"#;
        let data = parse_exiftool_json(json, "/tmp/x.jpg").unwrap();
        assert_eq!(data.tags.get("EXIF:ThumbnailImage"), Some(&None));
    }

    #[test]
    fn empty_array_returns_error() {
        let result = parse_exiftool_json("[]", "/tmp/x.jpg");
        assert!(result.is_err());
    }
}
