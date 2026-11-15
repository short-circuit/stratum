//! Kanban card rows for the [`super::PanelData`] seam.
//!
//! A row mirrors the shipped `KanbanBlockDto` payload of `get_kanban_blocks`
//! field for field, so a card edited on the board round-trips through
//! `update_block` without losing its tree links (`parent_id`/`left_id`), block
//! properties, collapse state or heading level. The board only renders the
//! content, marker, priority and page identity; the remaining fields exist so
//! the update path is lossless.

/// One task block shown on the Kanban board.
#[derive(Clone, Debug, PartialEq)]
pub struct KanbanCardRow {
    /// Block id (UUID).
    pub id: String,
    /// Block content (the card text).
    pub content: String,
    /// Parent block id, when the task is nested.
    pub parent_id: Option<String>,
    /// Left sibling block id, when the task is nested.
    pub left_id: Option<String>,
    /// Block properties, preserved verbatim on update.
    pub properties: Vec<(String, String)>,
    /// Task marker (`TODO`, `DOING`, `DONE`, `NOW`, `LATER`, `WAITING`, `CANCELLED`).
    pub marker: Option<String>,
    /// Priority (`A`, `B`, `C`), when set.
    pub priority: Option<String>,
    /// Whether the block is collapsed in the editor.
    pub collapsed: bool,
    /// Heading level of the block, when it is a heading.
    pub heading_level: Option<u8>,
    /// Vault-relative page path the block lives on.
    pub page_path: String,
    /// Page title, when the front matter carries one.
    pub page_title: Option<String>,
}

impl KanbanCardRow {
    /// Build a card with no tree links, properties or priority.
    ///
    /// Used by provider defaults and by the fixture's create flow so callers
    /// never spell out every optional field.
    pub fn new(id: &str, content: &str, marker: &str, page_path: &str) -> Self {
        Self {
            id: id.to_string(),
            content: content.to_string(),
            parent_id: None,
            left_id: None,
            properties: Vec::new(),
            marker: Some(marker.to_string()),
            priority: None,
            collapsed: false,
            heading_level: None,
            page_path: page_path.to_string(),
            page_title: None,
        }
    }
}
