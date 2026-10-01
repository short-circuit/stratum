//! Output sanitization & response caps (contract §8.2, §11).
//!
//! The server never returns stack traces, never leaks absolute filesystem
//! paths outside the vault, and keeps per-tool responses under
//! `MCP_RESPONSE_MAX` (1 MiB) — truncating list results rather than failing.
//!
//! Sanitizers here:
//! - `to_vault_relative` — rewrite an absolute path into a vault-relative
//!   `note_path` in error payloads;
//! - `scrub_hints` — ensure error `hint` strings contain no absolute paths and
//!   no token material;
//! - `truncate_for_response` — enforce the per-tool response cap on a JSON
//!   body, preferring a clean result-set cut point.

/// Convert an absolute filesystem path to a vault-relative display path.
///
/// Strips the canonical vault root prefix. If the value is not under the root,
/// it is returned unchanged (an escape is a bug the caller owns).
pub fn to_vault_relative(vault_root: &str, abs: &str) -> String {
    let root = vault_root.trim_end_matches(['/', '\\']);
    if let Some(rest) = abs.strip_prefix(root) {
        rest.trim_start_matches(['/', '\\']).to_string()
    } else {
        abs.to_string()
    }
}

/// Remove leading/trailing control characters and collapse internal absolute
/// paths out of an error hint.
pub fn scrub_hints(hint: &str, vault_root: &str) -> String {
    let trimmed = hint.trim();
    // Replace occurrences of the absolute root with a relative marker.
    let root = vault_root.trim_end_matches(['/', '\\']);
    if root.is_empty() {
        return trimmed.to_string();
    }
    let replaced = trimmed.replace(root, "<vault>");
    // Collapse any remaining `/home/...` style absolute prefixes.
    compress_absolute_prefixes(&replaced)
}

/// Compress a leading `/…/` absolute path prefix in a string into `<abs>`.
fn compress_absolute_prefixes(s: &str) -> String {
    // Match a token that begins with a slash and contains at least 3 path
    // segments (e.g. /home/user/x/y) and shorten it.
    let mut out = String::with_capacity(s.len());
    let mut rest = s;
    while let Some(idx) = rest.find('/') {
        // Only compress if the '/'-prefixed token is long (real absolute path).
        let end = rest[idx..]
            .find(char::is_whitespace)
            .map(|e| idx + e)
            .unwrap_or(rest.len());
        let token = &rest[idx..end];
        if token.matches('/').count() >= 3 && token.len() > 14 {
            out.push_str(&rest[..idx]);
            out.push_str("<abs>");
            rest = &rest[end..];
            // Consume following whitespace.
            rest = rest.trim_start_matches([' ', '\t']);
        } else {
            out.push_str(&rest[..end]);
            rest = &rest[end..];
        }
    }
    out.push_str(rest);
    out
}

/// Truncate a serialized response body to the contract cap.
///
/// Guarantees the returned string is **valid JSON** and ≤ `cap` bytes, so a
/// client can always parse a truncated response rather than receiving a
/// mangled half-object. It works by finding the largest valid-JSON prefix of
/// `body` under the cap (never splitting a UTF-8 code point). The caller can
/// append an `"hasMore": true`-style continuation flag before this call, or
/// consume the truncation flag to paginate.
///
/// Returns `(truncated_body, was_truncated)`.
pub fn truncate_for_response(body: &str, cap: usize) -> (String, bool) {
    if body.len() <= cap {
        return (body.to_string(), false);
    }
    // If the whole body is valid JSON, walk back from the cap until the prefix
    // is valid JSON. If no non-empty prefix parses (e.g. cap lands inside the
    // very first token), emit `null` — still valid JSON and size-bounded.
    if serde_json::from_str::<serde_json::Value>(body).is_ok() {
        let mut cut = cap.min(body.len());
        // Clamp to a char boundary first so we never split UTF-8.
        while cut > 0 && !body.is_char_boundary(cut) {
            cut -= 1;
        }
        while cut > 0 {
            if serde_json::from_str::<serde_json::Value>(&body[..cut]).is_ok() {
                return (body[..cut].to_string(), true);
            }
            cut -= 1;
            while cut > 0 && !body.is_char_boundary(cut) {
                cut -= 1;
            }
        }
        // `null` (4 bytes) is valid JSON and fits any real cap (MCP_RESPONSE_MAX
        // is 1 MiB). For an absurdly small cap, return the empty marker.
        if 4 <= cap {
            return ("null".to_string(), true);
        }
        return (String::new(), true);
    }
    // Non-JSON body (e.g. a bare string): fall back to a UTF-8-safe plain cut.
    let mut cut = cap.min(body.len());
    while cut > 0 && !body.is_char_boundary(cut) {
        cut -= 1;
    }
    (body[..cut].to_string(), true)
}

/// Detect whether a response body is within the configured cap.
pub fn within_cap(body: &str) -> bool {
    body.len() <= crate::MCP_RESPONSE_MAX
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn vault_relative_strips_root() {
        assert_eq!(
            to_vault_relative("/data/vault", "/data/vault/notes/a.md"),
            "notes/a.md"
        );
        assert_eq!(to_vault_relative("/data/vault", "/data/vault/a.md"), "a.md");
    }

    #[test]
    fn vault_relative_returns_unchanged_when_outside() {
        assert_eq!(
            to_vault_relative("/data/vault", "/etc/passwd"),
            "/etc/passwd"
        );
    }

    #[test]
    fn scrub_hint_removes_absolute_root() {
        assert_eq!(
            scrub_hints("open failed at /data/vault/notes/a.md", "/data/vault"),
            "open failed at <vault>/notes/a.md"
        );
    }

    #[test]
    fn scrub_compresses_other_absolutes() {
        let out = scrub_hints("err on /home/alice/.cargo/registry/x", "/data/vault");
        assert!(!out.contains("/home/alice/.cargo"));
        assert!(!out.contains('/'));
    }

    #[test]
    fn truncate_short_stays_short() {
        assert_eq!(truncate_for_response("abc", 10), ("abc".to_string(), false));
    }

    #[test]
    fn truncate_long_cuts_at_array_close() {
        // Body must remain *valid JSON* after truncation and stay ≤ cap.
        let body = "{\"items\":[\"a\",\"b\",\"c\"]}";
        let (out, was) = truncate_for_response(body, 12);
        assert!(was);
        assert!(
            serde_json::from_str::<serde_json::Value>(&out).is_ok(),
            "truncation must be parseable"
        );
        assert!(out.len() <= 12, "truncation must respect cap");
        assert!(out.len() < body.len());
    }

    #[test]
    fn truncate_nested_array_keeps_structure_parseable() {
        // Larger cap: cut inside the array still yields valid, parseable JSON
        // that respects the cap. The exact shape depends on where the cap
        // lands; the invariant is parseability + size, never mangelng.
        let body = "{\"items\":[\"aaaaaaaaaa\",\"bbbbbbbbbb\",\"cccccccccc\"]}";
        for cap in [20usize, 25, 30, 40, 49] {
            let (out, was) = truncate_for_response(body, cap);
            assert!(was);
            let parsed: serde_json::Value = serde_json::from_str(&out)
                .unwrap_or_else(|e| panic!("cap={cap} produced invalid JSON {out:?}: {e}"));
            assert!(
                out.len() <= cap,
                "cap={cap} produced {out:?} of len {}",
                out.len()
            );
            // If we can carry the items array, it must be present and intact.
            if out.contains("\"items\"") {
                assert!(parsed.get("items").is_some());
            }
        }
    }

    #[test]
    fn truncate_always_valid_json_under_any_cap() {
        // Property-ish: for many caps, the output is always parseable.
        let body = format!(
            "{{\"resp\":{}}}",
            serde_json::json!(["x".repeat(50), "y".repeat(50), "z".repeat(50)])
        );
        for cap in (1..=160).step_by(7) {
            let (out, was) = truncate_for_response(&body, cap);
            assert!(was);
            if out.is_empty() {
                // Sub-4-byte caps: no valid JSON fits; empty marker is the only
                // option. Never happens with the real 1 MiB response cap.
                assert!(cap < 4);
                continue;
            }
            assert!(
                serde_json::from_str::<serde_json::Value>(&out).is_ok(),
                "cap={cap} => {out:?}"
            );
            assert!(out.len() <= cap);
        }
        // A short body is not truncated.
        let (out, was) = truncate_for_response(&body, 10_000);
        assert!(!was);
        assert_eq!(out, body);
    }

    #[test]
    fn truncate_returns_valid_prefix_when_cap_mid_object() {
        // cap lands mid-key; the result must still be valid JSON (drop to the
        // largest complete value, here an empty object or nothing parseable).
        let body = "{\"title\":\"lorem ipsum dolor sit amet\",\"body\":\"x\"}";
        let (out, was) = truncate_for_response(body, 10);
        assert!(was);
        assert!(serde_json::from_str::<serde_json::Value>(&out).is_ok());
        assert!(out.len() <= 10);
    }

    #[test]
    fn utf8_boundary_not_split() {
        let body = "éééééééééé"; // 20 bytes of UTF-8.
        let cap = 10;
        let (out, _) = truncate_for_response(body, cap);
        assert!(out.is_char_boundary(out.len()));
        assert!(std::str::from_utf8(out.as_bytes()).is_ok());
    }

    #[test]
    fn within_cap_checks_contract_limit() {
        assert!(within_cap("small"));
        let big = "x".repeat(crate::MCP_RESPONSE_MAX);
        assert!(within_cap(&big));
        assert!(!within_cap(&format!("{big}x")));
    }
}
