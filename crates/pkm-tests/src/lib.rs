//! Integration tests for the Stratum PKM Rust crates.
//!
//! Integration-level tests live in `tests/`. This library crate intentionally
//! has no code — it exists so the workspace build includes `pkm-tests` as a
//! member and its integration tests run with `cargo test -p pkm-tests`.

/// Assert that a string is structurally well-formed HTML: it starts with a
/// doctype, has a matching pair of `<html>`…`</html>` and `<body>`…`</body>`
/// tags, and contains no NUL control characters. Used by the E7.F5 export
/// acceptance tests to prove exported files open cleanly without pulling in a
/// real HTML parser/browser dependency.
pub fn assert_html_well_formed(html: &str, label: &str) {
    assert!(
        html.trim_start().starts_with("<!DOCTYPE html>"),
        "{label}: must start with an HTML5 doctype"
    );
    let opens = html.matches("<html").count();
    let closes = html.matches("</html>").count();
    assert_eq!(
        opens, closes,
        "{label}: balanced <html> tags (open={opens}, close={closes})"
    );
    let body_opens = html.matches("<body").count();
    let body_closes = html.matches("</body>").count();
    assert_eq!(
        body_opens, body_closes,
        "{label}: balanced <body> tags (open={body_opens}, close={body_closes})"
    );
    assert!(
        !html.contains('\u{0000}'),
        "{label}: no NUL control characters"
    );
}
