use super::daemon::ResilientExifToolDaemon;

/// A single field write targeting a specific ExifTool tag.
pub struct MetadataWrite {
    /// Namespace-qualified tag name, e.g. `"EXIF:ImageDescription"`.
    pub tag: String,
    /// New value.  `None` clears (deletes) the tag.
    pub value: Option<String>,
}

/// Outcome of a write operation.
pub struct WriteResult {
    pub success: bool,
    pub message: String,
}

/// Write one or more metadata tags to `path` using the provided daemon.
///
/// Tags are batched into a single ExifTool invocation with
/// `-overwrite_original_in_place` and `-preserve`.
pub async fn write_metadata(
    daemon: &ResilientExifToolDaemon,
    path: &str,
    writes: &[MetadataWrite],
) -> Result<WriteResult, String> {
    if writes.is_empty() {
        return Ok(WriteResult {
            success: true,
            message: "No writes requested".to_string(),
        });
    }

    // Build tag arguments: "-Tag=Value" or "-Tag=" to clear.
    let tag_args: Vec<String> = writes
        .iter()
        .map(|w| match &w.value {
            Some(v) => format!("-{}={}", w.tag, v),
            None => format!("-{}=", w.tag),
        })
        .collect();

    let output = daemon.write_tags(path, &tag_args).await?;
    parse_write_output(&output)
}

/// Parse the stdout from an ExifTool write operation to determine success.
///
/// ExifTool prints lines like:
/// - `"1 image files updated"` — success
/// - `"0 image files updated"` — nothing changed (treat as success)
/// - `"Error: …"` — failure
fn parse_write_output(output: &str) -> Result<WriteResult, String> {
    for line in output.lines() {
        let lower = line.to_lowercase();
        if lower.contains("error") || lower.contains("warning: error") {
            return Ok(WriteResult {
                success: false,
                message: line.to_string(),
            });
        }
        if lower.contains("image files updated")
            || lower.contains("image file updated")
            || lower.contains("files updated")
            || lower.contains("file updated")
        {
            return Ok(WriteResult {
                success: true,
                message: line.trim().to_string(),
            });
        }
    }

    // If ExifTool produced no recognisable output, treat it as success
    // (e.g. when the value was already set).
    Ok(WriteResult {
        success: true,
        message: output.trim().to_string(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_success_output() {
        let r = parse_write_output("1 image files updated").unwrap();
        assert!(r.success);
    }

    #[test]
    fn parses_error_output() {
        let r = parse_write_output("Error: File not found - /tmp/missing.jpg").unwrap();
        assert!(!r.success);
    }

    #[test]
    fn empty_output_treated_as_success() {
        let r = parse_write_output("").unwrap();
        assert!(r.success);
    }
}
