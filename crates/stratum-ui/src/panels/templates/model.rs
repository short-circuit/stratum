//! Pure template-panel helpers: variable rows and their filtering.
//!
//! The shipped `TemplatesPanel` keeps one editable key/value pair per row and
//! filters out rows whose key is blank before calling `apply_template`. The
//! filtering lives here so it can be unit-tested without a window.

/// One editable template variable row.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct VariableRow {
    /// Placeholder name (without braces).
    pub key: String,
    /// Replacement value.
    pub value: String,
}

/// Drop rows whose key is blank, keeping the trimmed key and the raw value.
///
/// This mirrors the shipped `variables.filter(([k]) => k.trim())` before
/// `applyTemplate`. Duplicate keys are preserved in row order; the backend
/// applies them in the order given, so the last replacement wins.
pub fn filter_variables(rows: &[VariableRow]) -> Vec<(String, String)> {
    rows.iter()
        .filter(|row| !row.key.trim().is_empty())
        .map(|row| (row.key.trim().to_string(), row.value.clone()))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn blank_keys_are_dropped_and_keys_trimmed() {
        let rows = vec![
            VariableRow {
                key: " title ".to_string(),
                value: "Weekly review".to_string(),
            },
            VariableRow {
                key: "   ".to_string(),
                value: "ignored".to_string(),
            },
            VariableRow {
                key: String::new(),
                value: String::new(),
            },
            VariableRow {
                key: "date".to_string(),
                value: "2026-11-10".to_string(),
            },
        ];
        assert_eq!(
            filter_variables(&rows),
            vec![
                ("title".to_string(), "Weekly review".to_string()),
                ("date".to_string(), "2026-11-10".to_string()),
            ]
        );
    }

    #[test]
    fn no_rows_filter_to_no_variables() {
        assert!(filter_variables(&[]).is_empty());
    }
}
