//! Whiteboard deep links: `Board#Element` parsing and query-string encoding.
//!
//! A link may target a board, an element id, or a frame name. Resolution
//! against a loaded [`BoardDocument`] produces the ids to focus and the route
//! used by the shell (`/whiteboards?board=...&element=...`). Percent encoding
//! is implemented locally so the crate does not need an extra dependency;
//! everything outside `A-Za-z0-9-_.~` is encoded as uppercase `%XX`, and
//! decoding accepts `%XX` plus `+` as a space.

use std::fmt;

use pkm_core::{PkmError, PkmResult};

use crate::document::BoardDocument;
use crate::element_kind::ElementKind;

/// A parsed `Board` or `Board#Element` link.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BoardLink {
    /// Board name (file stem without the `.excalidraw` extension).
    pub board: String,
    /// Optional element id, frame name or text target.
    pub element: Option<String>,
}

impl BoardLink {
    /// Parse `Board`, `Board#Element` or `Board#Frame Name`.
    ///
    /// Whitespace around the board and element is trimmed; an empty board is
    /// rejected and an empty element segment is treated as absent.
    ///
    /// # Errors
    ///
    /// Returns [`PkmError::Validation`] when the board part is empty.
    pub fn parse(s: &str) -> PkmResult<Self> {
        let trimmed = s.trim();
        let (board, element) = match trimmed.split_once('#') {
            Some((board, element)) => (board.trim(), Some(element.trim())),
            None => (trimmed, None),
        };
        if board.is_empty() {
            return Err(PkmError::Validation("empty board link".to_string()));
        }
        let element = element
            .filter(|value| !value.is_empty())
            .map(str::to_string);
        Ok(Self {
            board: board.to_string(),
            element,
        })
    }

    /// Query string (`board=<encoded>&element=<encoded>`).
    ///
    /// The `element` parameter is omitted when the link has no element.
    pub fn to_query(&self) -> String {
        let mut query = format!("board={}", encode_component(&self.board));
        if let Some(element) = &self.element {
            if !element.is_empty() {
                query.push_str(&format!("&element={}", encode_component(element)));
            }
        }
        query
    }

    /// Parse a query string into a link.
    ///
    /// Returns `Ok(None)` when no `board` parameter is present.
    ///
    /// # Errors
    ///
    /// Returns [`PkmError::Validation`] when percent decoding fails or the
    /// board parameter is empty.
    pub fn from_query(query: &str) -> PkmResult<Option<Self>> {
        let query = query.strip_prefix('?').unwrap_or(query);
        let mut board = None;
        let mut element = None;
        for pair in query.split('&').filter(|pair| !pair.is_empty()) {
            let (key, value) = pair.split_once('=').unwrap_or((pair, ""));
            let key = decode_component(key)?;
            let value = decode_component(value)?;
            match key.as_str() {
                "board" => board = Some(value),
                "element" => element = Some(value),
                _ => {}
            }
        }
        let Some(board) = board else {
            return Ok(None);
        };
        if board.trim().is_empty() {
            return Err(PkmError::Validation("empty board query".to_string()));
        }
        let element = element
            .filter(|value| !value.trim().is_empty())
            .map(|value| value.trim().to_string());
        Ok(Some(Self { board, element }))
    }
}

/// Format the link back as `Board` or `Board#Element`.
impl fmt::Display for BoardLink {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match &self.element {
            Some(element) if !element.is_empty() => write!(f, "{}#{}", self.board, element),
            _ => f.write_str(&self.board),
        }
    }
}

/// Resolved deep-link target.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DeepLink {
    /// Board name.
    pub board: String,
    /// Element id to focus, when the link resolved to an element.
    pub element_id: Option<String>,
    /// Frame id to focus, when the link resolved to a frame.
    pub frame_id: Option<String>,
}

impl DeepLink {
    /// Resolve a [`BoardLink`] against a loaded document.
    ///
    /// The element segment may be an element id, a frame name
    /// (case-insensitive, trimmed), or an exact text match. A frame name
    /// resolves to `frame_id`; an element id resolves to `element_id` (and
    /// also `frame_id` when the element is a frame).
    ///
    /// # Errors
    ///
    /// Returns [`PkmError::NotFound`] when the element segment matches
    /// nothing in the document.
    pub fn resolve(link: &BoardLink, doc: &BoardDocument) -> PkmResult<Self> {
        let Some(target) = &link.element else {
            return Ok(Self {
                board: link.board.clone(),
                element_id: None,
                frame_id: None,
            });
        };
        if let Some(element) = doc.find_element_by_id(target) {
            if !element.is_deleted() {
                let is_frame = element.kind() == ElementKind::Frame;
                return Ok(Self {
                    board: link.board.clone(),
                    element_id: Some(element.id().to_string()),
                    frame_id: is_frame.then(|| element.id().to_string()),
                });
            }
        }
        if let Some(frame) = doc.find_frame_by_name(target) {
            return Ok(Self {
                board: link.board.clone(),
                element_id: None,
                frame_id: Some(frame.id().to_string()),
            });
        }
        let needle = target.trim().to_lowercase();
        let text_match = doc.visible_elements().find(|element| {
            element
                .text()
                .or_else(|| element.original_text())
                .map(|text| text.trim().to_lowercase() == needle)
                .unwrap_or(false)
        });
        match text_match {
            Some(element) => Ok(Self {
                board: link.board.clone(),
                element_id: Some(element.id().to_string()),
                frame_id: None,
            }),
            None => Err(PkmError::NotFound(format!(
                "element {target} in board {}",
                link.board
            ))),
        }
    }

    /// The `/whiteboards?board=...&element=...` route target.
    ///
    /// Frames are addressed through their id when no element id is present.
    pub fn route(&self) -> String {
        let element = self.element_id.clone().or_else(|| self.frame_id.clone());
        let link = BoardLink {
            board: self.board.clone(),
            element,
        };
        format!("/whiteboards?{}", link.to_query())
    }
}

/// Percent-encode one query component.
pub fn encode_component(value: &str) -> String {
    let mut out = String::with_capacity(value.len());
    for byte in value.as_bytes() {
        match byte {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => {
                out.push(char::from(*byte));
            }
            _ => out.push_str(&format!("%{byte:02X}")),
        }
    }
    out
}

/// Percent-decode one query component.
///
/// `+` decodes to a space.
///
/// # Errors
///
/// Returns [`PkmError::Validation`] for malformed `%XX` sequences or invalid
/// UTF-8.
pub fn decode_component(value: &str) -> PkmResult<String> {
    let bytes = value.as_bytes();
    let mut out = Vec::with_capacity(bytes.len());
    let mut index = 0;
    while index < bytes.len() {
        match bytes[index] {
            b'+' => {
                out.push(b' ');
                index += 1;
            }
            b'%' => {
                if index + 2 >= bytes.len() {
                    return Err(PkmError::Validation("truncated percent escape".to_string()));
                }
                let hex = std::str::from_utf8(&bytes[index + 1..index + 3])
                    .map_err(|e| PkmError::Validation(e.to_string()))?;
                let byte = u8::from_str_radix(hex, 16)
                    .map_err(|_| PkmError::Validation("invalid percent escape".to_string()))?;
                out.push(byte);
                index += 3;
            }
            byte => {
                out.push(byte);
                index += 1;
            }
        }
    }
    String::from_utf8(out).map_err(|e| PkmError::Validation(e.to_string()))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_and_formats_frame_links() {
        let link = BoardLink::parse("  Architecture # Frame One  ").unwrap();
        assert_eq!(link.board, "Architecture");
        assert_eq!(link.element.as_deref(), Some("Frame One"));
        assert_eq!(link.to_string(), "Architecture#Frame One");
    }

    #[test]
    fn query_round_trips_special_characters() {
        let link = BoardLink::parse("Board name#Frame & Co").unwrap();
        let query = link.to_query();
        assert!(query.contains("%20"));
        assert!(query.contains("%26"));
        assert_eq!(BoardLink::from_query(&query).unwrap(), Some(link));
    }

    #[test]
    fn empty_board_is_rejected() {
        assert!(BoardLink::parse("  #x ").is_err());
    }
}
