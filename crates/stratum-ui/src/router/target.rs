//! Route targets: a panel plus its decoded query parameters.
//!
//! The shipped React shell addresses deep links such as
//! `/whiteboards?board=Architecture%20sketch&element=fixture-flow-left`, so the
//! native router must keep the query string instead of discarding it. This
//! module parses that shape into [`RouteTarget`]; [`super::parse_path`]
//! delegates here and drops the parameters for callers that only need a panel.

/// A parsed route: the panel to open and its decoded query parameters.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RouteTarget {
    /// Panel addressed by the path portion of the link.
    pub panel: super::Panel,
    /// Decoded query parameters, in link order.
    pub query: Vec<(String, String)>,
}

impl RouteTarget {
    /// Look up the first value for a query parameter.
    pub fn param(&self, key: &str) -> Option<&str> {
        self.query
            .iter()
            .find(|(name, _)| name == key)
            .map(|(_, value)| value.as_str())
    }
}

/// Parse a link into a panel and its query parameters.
///
/// Returns `None` when the path portion does not match a known route, which
/// keeps unknown deep links from silently opening the journal. Query
/// components are percent-decoded and `+` is treated as a space, matching
/// `URLSearchParams` in the shipped web shell.
pub fn parse_target(target: &str) -> Option<RouteTarget> {
    let (path, query) = match target.split_once('?') {
        Some((path, query)) => (path, Some(query)),
        None => (target, None),
    };
    let panel = super::parse_panel_path(path)?;
    let query = query.map(parse_query).unwrap_or_default();
    Some(RouteTarget { panel, query })
}

/// Split a query string into decoded `(name, value)` pairs.
fn parse_query(query: &str) -> Vec<(String, String)> {
    query
        .split('&')
        .filter(|pair| !pair.is_empty())
        .map(|pair| {
            let (name, value) = pair.split_once('=').unwrap_or((pair, ""));
            (decode_query_component(name), decode_query_component(value))
        })
        .collect()
}

/// Decode one query component, treating `+` as an encoded space.
fn decode_query_component(component: &str) -> String {
    super::percent_decode(&component.replace('+', " "))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_plain_paths_without_query() {
        let target = parse_target("/whiteboards").unwrap();
        assert_eq!(target.panel, super::super::Panel::Whiteboards);
        assert!(target.query.is_empty());
    }

    #[test]
    fn rejects_unknown_paths() {
        assert!(parse_target("/nope?x=1").is_none());
    }

    #[test]
    fn decodes_query_values() {
        let target = parse_target("/whiteboards?board=My%20Board&tool=frame").unwrap();
        assert_eq!(target.param("board"), Some("My Board"));
        assert_eq!(target.param("tool"), Some("frame"));
        assert_eq!(target.param("missing"), None);
    }
}
