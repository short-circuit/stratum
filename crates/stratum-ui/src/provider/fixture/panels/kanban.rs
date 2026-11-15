//! Kanban seed data and state helpers for the deterministic fixture.
//!
//! Rust merges trait impls per crate, but a trait may only be implemented for a
//! type in a single `impl` block, so the fixture's [`PanelData`] methods stay in
//! [`super::impls`] and delegate here. The seed mirrors a real board: two To Do
//! cards, one In Progress card and two Done cards, one of them on today's
//! journal so the page link exercises the journal path. Mutations update the
//! in-memory list so the create/edit/delete/move flows are observable without a
//! vault.
//!
//! [`PanelData`]: crate::provider::panels::PanelData

use chrono::Utc;

use super::{panel_state, FixtureProvider};
use crate::provider::panels::KanbanCardRow;

/// Build one fixture card with a page title.
fn card(
    id: &str,
    content: &str,
    marker: &str,
    priority: Option<&str>,
    page_path: &str,
    page_title: &str,
) -> KanbanCardRow {
    let mut row = KanbanCardRow::new(id, content, marker, page_path);
    row.priority = priority.map(str::to_string);
    row.page_title = Some(page_title.to_string());
    row
}

/// Five deterministic task blocks across the three board columns.
///
/// `today` is the fixture clock's UTC date, matching the journal path the
/// service-backed provider creates new cards on.
pub(super) fn fixture_cards(today: &str) -> Vec<KanbanCardRow> {
    vec![
        card(
            "fixture-kanban-todo-1",
            "Draft the release notes",
            "TODO",
            Some("A"),
            "pages/Release Plan.md",
            "Release Plan",
        ),
        card(
            "fixture-kanban-todo-2",
            "Review the fixture outline",
            "LATER",
            None,
            &format!("journals/{today}.md"),
            today,
        ),
        card(
            "fixture-kanban-doing-1",
            "Wire the kanban board",
            "DOING",
            Some("B"),
            "pages/Native Shell.md",
            "Native Shell",
        ),
        card(
            "fixture-kanban-done-1",
            "Ship the settings surface",
            "DONE",
            None,
            "pages/Native Shell.md",
            "Native Shell",
        ),
        card(
            "fixture-kanban-done-2",
            "Archive the web frontend",
            "CANCELLED",
            Some("C"),
            "pages/Release Plan.md",
            "Release Plan",
        ),
    ]
}

/// The fixture's UTC date, used for journal paths.
fn today() -> String {
    Utc::now().format("%Y-%m-%d").to_string()
}

/// List the fixture's cards.
pub(super) fn cards(provider: &FixtureProvider) -> Result<Vec<KanbanCardRow>, String> {
    Ok(panel_state(provider)?.kanban.clone())
}

/// Create a card on today's journal, mirroring `create_kanban_block`.
pub(super) fn create(
    provider: &FixtureProvider,
    content: &str,
    marker: &str,
) -> Result<KanbanCardRow, String> {
    let mut state = panel_state(provider)?;
    let day = today();
    let id = format!("fixture-kanban-new-{}", state.kanban_created);
    state.kanban_created += 1;
    let mut row = KanbanCardRow::new(&id, content, marker, &format!("journals/{day}.md"));
    row.page_title = Some(day);
    state.kanban.push(row.clone());
    Ok(row)
}

/// Replace a card by id.
pub(super) fn update(provider: &FixtureProvider, card: KanbanCardRow) -> Result<(), String> {
    let mut state = panel_state(provider)?;
    let Some(existing) = state.kanban.iter_mut().find(|row| row.id == card.id) else {
        return Err(format!("unknown kanban card {}", card.id));
    };
    *existing = card;
    Ok(())
}

/// Remove a card by id.
pub(super) fn delete(provider: &FixtureProvider, id: &str) -> Result<(), String> {
    let mut state = panel_state(provider)?;
    let before = state.kanban.len();
    state.kanban.retain(|row| row.id != id);
    if state.kanban.len() == before {
        return Err(format!("unknown kanban card {id}"));
    }
    Ok(())
}
