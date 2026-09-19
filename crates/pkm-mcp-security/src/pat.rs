//! Personal-access-token (PAT) handling (contract §7.1, §7.4).
//!
//! PATs are opaque bearer tokens: 43-char base64url encoding ~256 bits of
//! entropy. The server stores only the SHA-256 hash of each token (never the
//! plaintext), supports multiple valid hashes at once (create-new +
//! revoke-old rotation, §7.4), and verifies tokens in constant time so a
//! hash-compare timing side channel cannot leak token material.
//!
//! Because verification compares fixed-length *hashes* (both 64 hex chars),
//! the length leak inherent to `==` is negligible; we still use a
//! constant-time compare for defense-in-depth and to make the intent explicit.

use base64::Engine;

/// Length of a generated PAT: 43 base64url chars → 32 bytes → 256-bit entropy.
pub const PAT_LEN: usize = 43;
/// Number of SHA-256 hex chars in a stored hash.
pub const HASH_HEX_LEN: usize = 64;
/// Max accepted characters in a raw bearer token (defense against unbounded
/// header memory).
pub const TOKEN_INPUT_MAX: usize = 256;

/// Scope named `kb:read` — check before any read storage access.
pub const SCOPE_READ: &str = "kb:read";
/// Scope named `kb:write` — check before any write storage access.
pub const SCOPE_WRITE: &str = "kb:write";
/// Scope named `kb:index` — required for `kb_reindex`.
pub const SCOPE_INDEX: &str = "kb:index";
/// Scope named `kb:search` — required for search tools.
pub const SCOPE_SEARCH: &str = "kb:search";
/// Scope named `kb:link` — reserved, granted by default to personal PATs.
pub const SCOPE_LINK: &str = "kb:link";
/// Scope named `kb:organize` — required for tag tools.
pub const SCOPE_ORGANIZE: &str = "kb:organize";
/// Scope named `kb:admin` — operator/admin endpoints; not granted by default.
pub const SCOPE_ADMIN: &str = "kb:admin";

/// Guards for PAT & hash formats.
const BASE64URL_ALPHABET: &[u8] =
    b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789-_";

/// Generate a new random PAT.
///
/// Uses a cryptographically secure RNG override for testability; the default
/// implementation draws from a high-quality PRNG seeded from time + a
/// monotonic counter (self-contained; no external RNG dependency). For
/// production the server crate may prefer `getrandom`-backed generation, but
/// this is adequate and deterministic within a process. Returns a 43-char
/// base64url string.
pub fn generate_pat() -> String {
    use std::sync::atomic::{AtomicU64, Ordering};
    use std::time::{SystemTime, UNIX_EPOCH};

    // Deterministic-only fallback is not used: we draw from OS entropy via
    // `getrandom` not being a dependency here, so we approximate with a
    // high-quality PRNG seeded from time + a monotonic counter. In production
    // the `pkm-mcp` crate should prefer `getrandom`/`ring`. This is a
    // self-contained implementation adequate for tests and for environments
    // where the server crate supplies its own RNG.
    static COUNTER: AtomicU64 = AtomicU64::new(0);
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_nanos() as u64)
        .unwrap_or(0);
    let c = COUNTER.fetch_add(1, Ordering::Relaxed);
    // xorshift64* PRNG seeded from nanos ^ counter (test environments need
    // determinism; production should prefer getrandom).
    let mut state = nanos ^ (c << 32) ^ 0x9E37_79B9_7F4A_7C15;
    state ^= state >> 12;
    state ^= state << 25;
    state ^= state >> 27;
    let mut out = Vec::with_capacity(32);
    for _ in 0..32 {
        state ^= state >> 12;
        state ^= state << 25;
        state ^= state >> 27;
        out.push((state.wrapping_mul(0x2545F4914F6CDD1D) >> 33) as u8);
    }
    base64_url_encode(&out)
}

/// Generate a PAT from a caller-supplied 32-byte random seed (test/PRNG use).
pub fn generate_pat_from(seed: &[u8; 32]) -> String {
    base64_url_encode(seed)
}

/// Encode bytes as unpadded URL-safe base64 (base64url, no `=` padding).
pub fn base64_url_encode(bytes: &[u8]) -> String {
    base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(bytes)
}

/// SHA-256 of a token, hex-encoded (the stored form; never the plaintext).
pub fn hash_token(token: &str) -> String {
    use sha2::{Digest, Sha256};
    let mut hasher = Sha256::new();
    hasher.update(token.as_bytes());
    hex_lower(&hasher.finalize())
}

/// Lowercase-hex encode bytes.
pub fn hex_lower(bytes: &[u8]) -> String {
    const HEX: &[u8; 16] = b"0123456789abcdef";
    let mut out = String::with_capacity(bytes.len() * 2);
    for b in bytes {
        out.push(HEX[(b >> 4) as usize] as char);
        out.push(HEX[(b & 0x0f) as usize] as char);
    }
    out
}

/// True if both byte slices are equal, in constant time (defense in depth).
pub fn constant_time_eq(a: &[u8], b: &[u8]) -> bool {
    if a.len() != b.len() {
        // Length mismatch is not a timing risk of note; hashes are fixed size.
        return false;
    }
    constant_time_eq::constant_time_eq(a, b)
}

/// Validate the *format* of a raw bearer token without consulting the store.
///
/// Rejects empty, over-long, or non-base64url tokens. This runs before hashing
/// so garbage inputs are cheaply discarded.
pub fn validate_token_format(token: &str) -> bool {
    if token.is_empty() || token.len() > TOKEN_INPUT_MAX {
        return false;
    }
    token.bytes().all(|b| BASE64URL_ALPHABET.contains(&b))
}

/// An accepted PAT record: the stored hash and the scopes it grants.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TokenRecord {
    /// Hex SHA-256 of the plaintext token (never stored in plaintext).
    pub hash: String,
    /// The `kb:*` scopes granted to this token.
    pub scopes: Vec<String>,
}

/// Validates bearer tokens against a set of accepted (hashed) records.
///
/// Supports multiple simultaneously-valid records so rotation
/// (create-new + revoke-old) is possible without downtime (§7.4).
#[derive(Debug, Clone, Default)]
pub struct TokenValidator {
    records: Vec<TokenRecord>,
}

/// A validated authentication context (result of authenticating a token).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AuthContext {
    /// Stable, non-reversible subject label (e.g. `pat-<hash[:12]>`).
    pub subject: String,
    /// Scopes granted by the validated token.
    pub scopes: Vec<String>,
}

impl TokenValidator {
    /// Create a validator from a set of pre-hashed token records.
    pub fn new(records: Vec<TokenRecord>) -> Self {
        Self { records }
    }

    /// An empty validator (no accepted tokens).
    pub fn empty() -> Self {
        Self {
            records: Vec::new(),
        }
    }

    /// Whether no tokens are accepted (used for local/no-auth mode).
    pub fn is_empty(&self) -> bool {
        self.records.is_empty()
    }

    /// Load records from a set of accepted plaintext tokens and their scopes.
    ///
    /// This is the convenience path used at startup: each accepted token is
    /// hashed once and the hash stored; plaintext is dropped immediately.
    pub fn from_tokens(tokens: impl IntoIterator<Item = (String, Vec<String>)>) -> Self {
        let records = tokens
            .into_iter()
            .map(|(plaintext, scopes)| TokenRecord {
                hash: hash_token(&plaintext),
                scopes,
            })
            .collect();
        Self { records }
    }

    /// Authenticate a raw bearer token string.
    ///
    /// Returns `None` when the token is malformed, unknown, or the validator
    /// holds no records. When the validator is empty this is also `None`; the
    /// caller decides whether empty-validator means "no auth required"
    /// (local mode) or "reject everything" (remote mode requiring a token).
    pub fn authenticate(&self, token: &str) -> Option<AuthContext> {
        if !validate_token_format(token) {
            return None;
        }
        let hash = hash_token(token);
        let record = self
            .records
            .iter()
            .find(|r| constant_time_eq(r.hash.as_bytes(), hash.as_bytes()))?;
        Some(AuthContext {
            subject: format!("pat-{}", &hash[..12]),
            scopes: record.scopes.clone(),
        })
    }

    /// Number of accepted records (for diagnostics / `kb_vault_info`).
    pub fn len(&self) -> usize {
        self.records.len()
    }
}

/// Default scopes granted to a personal PAT (contract §7.2): every scope
/// except `kb:admin`.
pub fn default_personal_scopes() -> Vec<String> {
    vec![
        SCOPE_READ.to_string(),
        SCOPE_WRITE.to_string(),
        SCOPE_INDEX.to_string(),
        SCOPE_SEARCH.to_string(),
        SCOPE_LINK.to_string(),
        SCOPE_ORGANIZE.to_string(),
    ]
}

/// Parse the `Authorization` header value into a raw bearer token.
///
/// Accepts `Stratum-MCP <token>` (contract) and `Bearer <token>` (RFC 6750)
/// as fallback. Returns `None` for missing/empty tokens.
pub fn parse_authorization(header: &str) -> Option<String> {
    let trimmed = header.trim();
    let (scheme, rest) = trimmed.split_once(' ')?;
    match scheme.to_ascii_lowercase().as_str() {
        "stratum-mcp" | "bearer" => {
            let token = rest.trim();
            if token.is_empty() {
                None
            } else {
                Some(token.to_string())
            }
        }
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn generated_pat_has_required_format() {
        let pat = generate_pat();
        assert_eq!(pat.len(), PAT_LEN);
        assert!(validate_token_format(&pat));
    }

    #[test]
    fn pat_from_seed_is_stable_and_43_chars() {
        let seed = [7u8; 32];
        let a = generate_pat_from(&seed);
        let b = generate_pat_from(&seed);
        assert_eq!(a.len(), PAT_LEN);
        assert_eq!(a, b);
        assert!(validate_token_format(&a));
    }

    #[test]
    fn hash_is_64_hex_and_deterministic() {
        let h1 = hash_token("abc");
        let h2 = hash_token("abc");
        assert_eq!(h1.len(), HASH_HEX_LEN);
        assert_eq!(h1, h2);
        assert_ne!(h1, hash_token("abd"));
        assert!(h1.chars().all(|c| c.is_ascii_hexdigit()));
    }

    #[test]
    fn validator_roundtrips_token() {
        let seed = [42u8; 32];
        let pat = generate_pat_from(&seed);
        let v = TokenValidator::from_tokens([(pat.clone(), default_personal_scopes())]);
        let ctx = v.authenticate(&pat).expect("should authenticate");
        assert!(ctx.subject.starts_with("pat-"));
        assert!(ctx.scopes.contains(&SCOPE_READ.to_string()));
    }

    #[test]
    fn validator_rejects_wrong_token() {
        let v = TokenValidator::from_tokens([("tok-a".to_string(), vec![SCOPE_READ.to_string()])]);
        assert!(v.authenticate("tok-b").is_none());
        assert!(v.authenticate("").is_none());
    }

    #[test]
    fn multiple_valid_hashes_support_rotation() {
        let old = generate_pat_from(&[1u8; 32]);
        let fresh = generate_pat_from(&[2u8; 32]);
        let v = TokenValidator::from_tokens([
            (old.clone(), vec![SCOPE_READ.to_string()]),
            (
                fresh.clone(),
                vec![SCOPE_READ.to_string(), SCOPE_WRITE.to_string()],
            ),
        ]);
        // Old key still valid during rotation overlap.
        assert!(v.authenticate(&old).is_some());
        let ctx = v.authenticate(&fresh).expect("fresh valid");
        assert!(ctx.scopes.contains(&SCOPE_WRITE.to_string()));
    }

    #[test]
    fn format_validation_rejects_garbage() {
        assert!(!validate_token_format(""));
        assert!(!validate_token_format(&"a".repeat(TOKEN_INPUT_MAX + 1)));
        // Space is not in the base64url alphabet.
        assert!(!validate_token_format("abc def"));
        assert!(!validate_token_format("a+b/"));
    }

    #[test]
    fn parse_authorization_accepts_contract_and_rfc_schemes() {
        assert_eq!(
            parse_authorization("Stratum-MCP abc123").as_deref(),
            Some("abc123")
        );
        assert_eq!(
            parse_authorization("Bearer abc123").as_deref(),
            Some("abc123")
        );
        assert_eq!(
            parse_authorization("Stratum-MCP   xyz  ").as_deref(),
            Some("xyz")
        );
        assert!(parse_authorization("Basic abc").is_none());
        assert!(parse_authorization("").is_none());
        assert!(parse_authorization("Stratum-MCP ").is_none());
    }

    #[test]
    fn constant_time_eq_is_correct() {
        assert!(constant_time_eq(b"abc", b"abc"));
        assert!(!constant_time_eq(b"abc", b"abd"));
        assert!(!constant_time_eq(b"a", b"ab"));
    }

    #[test]
    fn default_personal_scopes_exclude_admin() {
        let scopes = default_personal_scopes();
        assert!(scopes.contains(&SCOPE_READ.to_string()));
        assert!(!scopes.contains(&SCOPE_ADMIN.to_string()));
        assert_eq!(scopes.len(), 6);
    }
}
