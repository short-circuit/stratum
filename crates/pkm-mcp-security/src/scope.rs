//! Scope vocabulary and authorization checks (contract §7.2).
//!
//! Every MCP tool requires exactly one scope (§6 mapping table). Authorization
//! checks happen **before any storage access**; a request whose token lacks the
//! required scope is rejected with `ScopesDenied` (-32005).
//!
//! In local/stdio mode there is no authentication and every scope is
//! implicitly granted; authorization here is therefore a no-op for local
//! clients and enforced only for authenticated remote clients.

use crate::pat::AuthContext;

/// Result of a scope check.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ScopeDecision {
    /// The request holds the required scope (or auth is disabled).
    Allowed,
    /// The request is missing the required scope.
    Denied {
        /// The scope the tool required.
        required: String,
        /// The scopes actually granted to the request.
        granted: Vec<String>,
    },
}

/// Check whether an authenticated context holds `required_scope`.
///
/// `auth_disabled` is set for stdio/loopback mode where no token is expected
/// and all scope checks pass.
pub fn check_scope(auth: &AuthContext, required_scope: &str, auth_disabled: bool) -> ScopeDecision {
    if auth_disabled {
        return ScopeDecision::Allowed;
    }
    if auth.scopes.iter().any(|s| s == required_scope) {
        ScopeDecision::Allowed
    } else {
        ScopeDecision::Denied {
            required: required_scope.to_string(),
            granted: auth.scopes.clone(),
        }
    }
}

/// Convenience: Boolean form of [`check_scope`].
pub fn has_scope(auth: &AuthContext, required_scope: &str, auth_disabled: bool) -> bool {
    matches!(
        check_scope(auth, required_scope, auth_disabled),
        ScopeDecision::Allowed
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::pat::{default_personal_scopes, AuthContext};

    fn ctx(scopes: Vec<String>) -> AuthContext {
        AuthContext {
            subject: "pat-test".into(),
            scopes,
        }
    }

    #[test]
    fn read_scope_allows_read_tool() {
        let auth = ctx(default_personal_scopes());
        assert_eq!(
            check_scope(&auth, crate::pat::SCOPE_READ, false),
            ScopeDecision::Allowed
        );
    }

    #[test]
    fn missing_scope_is_denied() {
        // A read-only token without kb:write.
        let auth = ctx(vec![crate::pat::SCOPE_READ.to_string()]);
        assert_eq!(
            check_scope(&auth, crate::pat::SCOPE_WRITE, false),
            ScopeDecision::Denied {
                required: "kb:write".into(),
                granted: vec!["kb:read".into()],
            }
        );
    }

    #[test]
    fn auth_disabled_grants_everything() {
        let auth = AuthContext {
            subject: String::new(),
            scopes: vec![],
        };
        assert!(has_scope(&auth, crate::pat::SCOPE_ADMIN, true));
    }

    #[test]
    fn admin_scope_not_in_default_personal() {
        let auth = ctx(default_personal_scopes());
        assert!(!has_scope(&auth, crate::pat::SCOPE_ADMIN, false));
    }

    #[test]
    fn exact_match_not_prefix() {
        // kb:read must not satisfy kb:reader (hypothetical) — exact match only.
        let auth = ctx(vec!["kb:read".into()]);
        assert!(has_scope(&auth, "kb:read", false));
        assert!(!has_scope(&auth, "kb:reader", false));
    }
}
