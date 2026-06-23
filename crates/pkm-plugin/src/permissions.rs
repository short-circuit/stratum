use std::collections::HashSet;
use std::fmt;
use thiserror::Error;

/// Permission variants available to WASM plugins.
///
/// Each variant restricts a specific capability. The `All` variant grants
/// every permission (superuser).
#[derive(Debug, Clone, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
pub enum Permission {
    FileRead,
    FileWrite,
    Network,
    Git,
    Exec,
    All,
}

impl Permission {
    /// Parse a single permission string into a `Permission`.
    ///
    /// Supported values:
    /// - `"file:read"` → `FileRead`
    /// - `"file:write"` → `FileWrite`
    /// - `"network"` → `Network`
    /// - `"git"` → `Git`
    /// - `"exec"` → `Exec`
    /// - `"all"` → `All`
    pub fn parse(s: &str) -> Option<Self> {
        match s.trim() {
            "file:read" => Some(Self::FileRead),
            "file:write" => Some(Self::FileWrite),
            "network" => Some(Self::Network),
            "git" => Some(Self::Git),
            "exec" => Some(Self::Exec),
            "all" => Some(Self::All),
            _ => None,
        }
    }

    /// Return the canonical string representation.
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::FileRead => "file:read",
            Self::FileWrite => "file:write",
            Self::Network => "network",
            Self::Git => "git",
            Self::Exec => "exec",
            Self::All => "all",
        }
    }
}

impl fmt::Display for Permission {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

// ---------------------------------------------------------------------------
// PermissionSet
// ---------------------------------------------------------------------------

/// A set of granted permissions for a plugin.
///
/// When `All` is present, every [`check`](PermissionSet::check) returns `true`
/// and [`check_strict`](PermissionSet::check_strict) succeeds.
#[derive(Debug, Clone, Default, serde::Serialize, serde::Deserialize)]
pub struct PermissionSet {
    granted: HashSet<Permission>,
}

impl PermissionSet {
    /// Create an empty permission set.
    pub fn new() -> Self {
        Self {
            granted: HashSet::new(),
        }
    }

    /// Create a permission set from a slice of permissions.
    pub fn from_permissions(perms: &[Permission]) -> Self {
        let mut set = Self::new();
        for p in perms {
            set.grant(p.clone());
        }
        set
    }

    /// Parse a list of config strings (e.g. `["file:read", "network"]`) into a
    /// `PermissionSet`. Unknown strings are silently ignored.
    pub fn from_config_strings(strings: &[String]) -> Self {
        let mut set = Self::new();
        for s in strings {
            if let Some(p) = Permission::parse(s) {
                set.grant(p);
            }
        }
        set
    }

    /// Grant a permission.
    pub fn grant(&mut self, perm: Permission) {
        // Granting `All` clears individual entries — everything is implied.
        if perm == Permission::All {
            self.granted.clear();
            self.granted.insert(Permission::All);
            return;
        }
        // If `All` is already granted, individual additions are redundant.
        if self.granted.contains(&Permission::All) {
            return;
        }
        self.granted.insert(perm);
    }

    /// Revoke a permission. Revoking `All` clears the entire set.
    pub fn revoke(&mut self, perm: &Permission) {
        if perm == &Permission::All {
            self.granted.clear();
            return;
        }
        self.granted.remove(perm);
    }

    /// Check whether a permission is granted (non-strict).
    ///
    /// Returns `true` if the permission (or `All`) is present.
    pub fn check(&self, perm: &Permission) -> bool {
        if self.granted.contains(&Permission::All) {
            return true;
        }
        self.granted.contains(perm)
    }

    /// Strict permission check.
    ///
    /// Returns `Ok(())` if allowed, or a [`PermissionError`] describing the
    /// denial.
    pub fn check_strict(&self, perm: &Permission) -> Result<(), PermissionError> {
        if self.check(perm) {
            Ok(())
        } else {
            Err(PermissionError {
                required: perm.clone(),
                granted: self.clone(),
            })
        }
    }

    /// Return an iterator over the granted permissions.
    pub fn iter(&self) -> impl Iterator<Item = &Permission> {
        self.granted.iter()
    }

    /// Return the number of explicitly-granted permissions.
    pub fn len(&self) -> usize {
        self.granted.len()
    }

    /// Whether the set is empty.
    pub fn is_empty(&self) -> bool {
        self.granted.is_empty()
    }

    /// Convert this permission set to a human-readable string.
    pub fn as_str_list(&self) -> String {
        let perms: Vec<String> = self.granted.iter().map(|p| p.to_string()).collect();
        perms.join(", ")
    }
}

impl fmt::Display for PermissionSet {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let perms: Vec<String> = self.granted.iter().map(|p| p.to_string()).collect();
        if perms.is_empty() {
            write!(f, "(none)")
        } else {
            write!(f, "{}", perms.join(", "))
        }
    }
}

impl From<Vec<String>> for PermissionSet {
    fn from(strings: Vec<String>) -> Self {
        Self::from_config_strings(&strings)
    }
}

impl IntoIterator for PermissionSet {
    type Item = Permission;
    type IntoIter = std::collections::hash_set::IntoIter<Permission>;

    fn into_iter(self) -> Self::IntoIter {
        self.granted.into_iter()
    }
}

// ---------------------------------------------------------------------------
// PermissionError
// ---------------------------------------------------------------------------

/// Error returned when a plugin lacks a required permission.
#[derive(Error, Debug, Clone)]
#[error("Permission denied: required `{required}`, granted `{granted}`")]
pub struct PermissionError {
    pub required: Permission,
    pub granted: PermissionSet,
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_permission_strings() {
        assert_eq!(Permission::parse("file:read"), Some(Permission::FileRead));
        assert_eq!(Permission::parse("file:write"), Some(Permission::FileWrite));
        assert_eq!(Permission::parse("network"), Some(Permission::Network));
        assert_eq!(Permission::parse("git"), Some(Permission::Git));
        assert_eq!(Permission::parse("exec"), Some(Permission::Exec));
        assert_eq!(Permission::parse("all"), Some(Permission::All));
        assert_eq!(Permission::parse("unknown"), None);
        assert_eq!(Permission::parse(""), None);
    }

    #[test]
    fn test_permission_display() {
        assert_eq!(Permission::FileRead.to_string(), "file:read");
        assert_eq!(Permission::All.to_string(), "all");
    }

    #[test]
    fn test_permission_set_grant_check() {
        let mut set = PermissionSet::new();
        assert!(!set.check(&Permission::FileRead));
        assert!(!set.check(&Permission::Network));

        set.grant(Permission::FileRead);
        assert!(set.check(&Permission::FileRead));
        assert!(!set.check(&Permission::Network));
    }

    #[test]
    fn test_permission_set_all_grants_everything() {
        let mut set = PermissionSet::new();
        set.grant(Permission::All);

        assert!(set.check(&Permission::FileRead));
        assert!(set.check(&Permission::FileWrite));
        assert!(set.check(&Permission::Network));
        assert!(set.check(&Permission::Git));
        assert!(set.check(&Permission::Exec));
    }

    #[test]
    fn test_permission_set_revoke() {
        let mut set = PermissionSet::new();
        set.grant(Permission::FileRead);
        set.grant(Permission::Network);
        assert!(set.check(&Permission::Network));

        set.revoke(&Permission::Network);
        assert!(!set.check(&Permission::Network));
        assert!(set.check(&Permission::FileRead));
    }

    #[test]
    fn test_permission_set_revoke_all() {
        let mut set = PermissionSet::new();
        set.grant(Permission::All);
        assert!(set.check(&Permission::Exec));

        set.revoke(&Permission::All);
        assert!(!set.check(&Permission::Exec));
        assert!(!set.check(&Permission::FileRead));
        assert!(set.is_empty());
    }

    #[test]
    fn test_check_strict_ok() {
        let mut set = PermissionSet::new();
        set.grant(Permission::Git);
        assert!(set.check_strict(&Permission::Git).is_ok());
    }

    #[test]
    fn test_check_strict_error() {
        let set = PermissionSet::new();
        let err = set.check_strict(&Permission::FileWrite).unwrap_err();
        assert_eq!(err.required, Permission::FileWrite);
        assert!(err.to_string().contains("Permission denied"));
    }

    #[test]
    fn test_check_strict_with_all() {
        let mut set = PermissionSet::new();
        set.grant(Permission::All);
        assert!(set.check_strict(&Permission::Network).is_ok());
        assert!(set.check_strict(&Permission::Exec).is_ok());
    }

    #[test]
    fn test_permission_set_from_config_strings() {
        let strings = vec![
            "file:read".to_string(),
            "network".to_string(),
            "bogus".to_string(), // should be silently ignored
        ];
        let set = PermissionSet::from_config_strings(&strings);
        assert!(set.check(&Permission::FileRead));
        assert!(set.check(&Permission::Network));
        assert!(!set.check(&Permission::FileWrite));
        assert!(!set.check(&Permission::Git));
    }

    #[test]
    fn test_permission_set_from_slice() {
        let set = PermissionSet::from_permissions(&[Permission::FileRead, Permission::FileWrite]);
        assert!(set.check(&Permission::FileRead));
        assert!(set.check(&Permission::FileWrite));
        assert!(!set.check(&Permission::Network));
        assert_eq!(set.len(), 2);
    }

    #[test]
    fn test_permission_set_into_iterator() {
        let set = PermissionSet::from_permissions(&[Permission::Git, Permission::Exec]);
        let perms: Vec<Permission> = set.into_iter().collect();
        assert_eq!(perms.len(), 2);
        assert!(perms.contains(&Permission::Git));
        assert!(perms.contains(&Permission::Exec));
    }

    #[test]
    fn test_permission_set_default_is_empty() {
        let set = PermissionSet::default();
        assert!(set.is_empty());
        assert!(!set.check(&Permission::FileRead));
    }

    #[test]
    fn test_grant_all_clears_individual_perms() {
        let mut set = PermissionSet::new();
        set.grant(Permission::FileRead);
        set.grant(Permission::Network);
        set.grant(Permission::All);
        // After granting all, individual entries are cleared
        assert_eq!(set.len(), 1);
        assert!(set.check(&Permission::FileRead));
    }

    #[test]
    fn test_grant_individual_after_all_is_noop() {
        let mut set = PermissionSet::new();
        set.grant(Permission::All);
        set.grant(Permission::FileRead); // no-op since All already present
        assert_eq!(set.len(), 1);
    }
}
