//! Native Kanban board panel.
//!
//! [`KanbanView`] ports the shipped desktop `KanbanPanel`: a three-column board
//! (To Do / In Progress / Done) over the vault's task-marked blocks, a Done
//! column that can collapse, inline card creation, drag-and-drop between
//! columns, a card context menu (edit/delete), the edit dialog (content, marker,
//! priority) and the "Today's Journal" shortcut.
//!
//! Cards are projections of Markdown blocks: every mutation goes through the
//! [`crate::provider::PanelData`] seam, which rewrites the owning page so the
//! board and the notes stay in sync. Column placement is derived from the task
//! marker ([`model::column_for_marker`]); dropping a card rewrites its marker to
//! the column's canonical marker.
//!
//! [`KANBAN_I18N_KEYS`] is the release gate list for every string this surface
//! renders.

use std::sync::Arc;

use gpui_kit::component::input::{InputEvent, TextareaState};
use gpui_kit::component::ActiveTheme;
use gpui_kit::prelude::*;
use gpui_kit::*;
use stratum_i18n::t_global;

use crate::panels::PanelRequests;
use crate::provider::panels::KanbanCardRow;
use crate::provider::ShellData;

pub mod board;
pub mod card;
pub mod edit;
pub mod model;
pub mod widgets;

use model::ColumnId;

/// Every translation key the Kanban surface renders.
///
/// The contract test walks this list against all eight catalogs, so adding a
/// visible string without a catalog entry fails the build instead of leaking a
/// raw key into the UI.
pub const KANBAN_I18N_KEYS: &[&str] = &[
    "kanban.add",
    "kanban.add_card",
    "kanban.cancel",
    "kanban.delete",
    "kanban.done",
    "kanban.edit",
    "kanban.in_progress",
    "kanban.kanban",
    "kanban.no_tasks_yet",
    "kanban.refresh",
    "kanban.task_description",
    "kanban.to_do",
    "kanban.today_apos_s_journal",
    "kanban_edit_dialog.cancel",
    "kanban_edit_dialog.content",
    "kanban_edit_dialog.edit_card",
    "kanban_edit_dialog.marker",
    "kanban_edit_dialog.none",
    "kanban_edit_dialog.priority",
    "kanban_edit_dialog.save",
    "flashcards.source",
];

/// Deterministic start state for the capture harness.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct KanbanStart {
    /// Start with the Done column collapsed.
    pub done_collapsed: bool,
    /// Open the inline "add card" form in this column.
    pub new_card: Option<ColumnId>,
    /// Open the edit dialog for this card id.
    pub editing: Option<String>,
    /// Open the context menu for this card id.
    pub menu: Option<String>,
}

/// The Kanban board panel.
pub struct KanbanView {
    provider: Arc<dyn ShellData>,
    cards: Vec<KanbanCardRow>,
    load_error: Option<String>,
    message: Option<String>,
    done_collapsed: bool,
    new_card_column: Option<ColumnId>,
    new_card_text: String,
    new_card_input: Entity<TextareaState>,
    _new_card_subscription: Subscription,
    editing: Option<edit::EditState>,
    edit_input: Entity<TextareaState>,
    _edit_subscription: Subscription,
    menu_card: Option<String>,
    drag_target: Option<ColumnId>,
    pending_navigation: Option<String>,
}

impl KanbanView {
    /// Build the panel and load the board.
    pub fn new(
        provider: Arc<dyn ShellData>,
        start: KanbanStart,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> Self {
        let placeholder = t_global("kanban.task_description");
        let new_card_input = cx.new(|cx| {
            TextareaState::new(window, cx)
                .placeholder(placeholder)
                .rows(2)
        });
        let new_card_subscription = cx.subscribe(
            &new_card_input,
            |this: &mut Self, input, event: &InputEvent, cx| {
                if matches!(event, InputEvent::Change) {
                    this.new_card_text = input.read(cx).value().to_string();
                    cx.notify();
                }
            },
        );
        let edit_placeholder = t_global("kanban_edit_dialog.content");
        let edit_input = cx.new(|cx| {
            TextareaState::new(window, cx)
                .placeholder(edit_placeholder)
                .rows(4)
        });
        let edit_subscription = cx.subscribe(
            &edit_input,
            |this: &mut Self, input, event: &InputEvent, cx| {
                if matches!(event, InputEvent::Change) {
                    let value = input.read(cx).value().to_string();
                    if let Some(editing) = this.editing.as_mut() {
                        editing.card.content = value;
                    }
                    cx.notify();
                }
            },
        );
        let mut view = Self {
            provider,
            cards: Vec::new(),
            load_error: None,
            message: None,
            done_collapsed: start.done_collapsed,
            new_card_column: start.new_card,
            new_card_text: String::new(),
            new_card_input,
            _new_card_subscription: new_card_subscription,
            editing: None,
            edit_input,
            _edit_subscription: edit_subscription,
            menu_card: start.menu.clone(),
            drag_target: None,
            pending_navigation: None,
        };
        view.reload();
        if start.new_card.is_some() {
            view.focus_new_card(window, cx);
        }
        if let Some(id) = start.editing.as_deref() {
            view.begin_edit(id, window, cx);
        }
        view
    }

    /// The loaded cards.
    pub fn cards(&self) -> &[KanbanCardRow] {
        &self.cards
    }

    /// The board load error, when the provider refused the query.
    pub fn load_error(&self) -> Option<&str> {
        self.load_error.as_deref()
    }

    /// The last action error or status message.
    pub fn message(&self) -> Option<&str> {
        self.message.as_deref()
    }

    /// Whether the Done column is collapsed.
    pub fn done_collapsed(&self) -> bool {
        self.done_collapsed
    }

    /// The column whose inline add form is open.
    pub fn new_card_column(&self) -> Option<ColumnId> {
        self.new_card_column
    }

    /// The inline add form's current text.
    pub fn new_card_text(&self) -> &str {
        &self.new_card_text
    }

    /// The inline add form's input entity.
    pub fn new_card_input(&self) -> &Entity<TextareaState> {
        &self.new_card_input
    }

    /// The open edit dialog, if any.
    pub fn editing(&self) -> Option<&edit::EditState> {
        self.editing.as_ref()
    }

    /// The edit dialog's content input entity.
    pub fn edit_input(&self) -> &Entity<TextareaState> {
        &self.edit_input
    }

    /// The card whose context menu is open, if any.
    pub fn menu_card(&self) -> Option<&str> {
        self.menu_card.as_deref()
    }

    /// The column highlighted as the current drop target.
    pub fn drag_target(&self) -> Option<ColumnId> {
        self.drag_target
    }

    /// Reload the board from the provider.
    pub fn reload(&mut self) {
        match self.provider.kanban_cards() {
            Ok(cards) => {
                self.cards = cards;
                self.load_error = None;
            }
            Err(error) => {
                self.cards.clear();
                self.load_error = Some(error);
            }
        }
    }

    /// Reload on demand (the board's Refresh action).
    pub fn refresh(&mut self, cx: &mut Context<Self>) {
        self.message = None;
        self.reload();
        cx.notify();
    }

    /// Collapse or expand the Done column.
    pub fn toggle_done(&mut self, cx: &mut Context<Self>) {
        self.done_collapsed = !self.done_collapsed;
        cx.notify();
    }

    /// Clear the inline add form.
    pub fn cancel_new_card(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        self.new_card_column = None;
        self.new_card_text.clear();
        let input = self.new_card_input.clone();
        input.update(cx, |input, cx| input.set_value(String::new(), window, cx));
        cx.notify();
    }

    /// Open the inline add form in `column` and focus it.
    pub fn start_new_card(
        &mut self,
        column: ColumnId,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.new_card_column = Some(column);
        self.new_card_text.clear();
        self.focus_new_card(window, cx);
        cx.notify();
    }

    /// Focus (and clear) the inline add form.
    fn focus_new_card(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        self.new_card_text.clear();
        let input = self.new_card_input.clone();
        input.update(cx, |input, cx| {
            input.set_value(String::new(), window, cx);
            input.focus(window, cx);
        });
    }

    /// Create the card typed into the inline form.
    ///
    /// Empty input is a no-op, matching the shipped panel's disabled Add
    /// button; a provider failure keeps the form open so the text is not lost.
    pub fn add_card(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let Some(column) = self.new_card_column else {
            return;
        };
        let content = self.new_card_text.trim().to_string();
        if content.is_empty() {
            return;
        }
        match self
            .provider
            .create_kanban_card(&content, column.target_marker())
        {
            Ok(_) => {
                self.reload();
                self.new_card_column = None;
                self.new_card_text.clear();
                let input = self.new_card_input.clone();
                input.update(cx, |input, cx| input.set_value(String::new(), window, cx));
            }
            Err(error) => self.message = Some(error),
        }
        cx.notify();
    }

    /// Move a card to `column` by rewriting its marker.
    pub fn move_card(&mut self, id: &str, column: ColumnId, cx: &mut Context<Self>) {
        let Some(card) = self.cards.iter().find(|card| card.id == id).cloned() else {
            return;
        };
        if card.marker.as_deref().and_then(model::column_for_marker) == Some(column) {
            return;
        }
        let mut updated = card;
        updated.marker = Some(column.target_marker().to_string());
        match self.provider.update_kanban_card(updated) {
            Ok(()) => self.reload(),
            Err(error) => {
                self.message = Some(error);
                self.reload();
            }
        }
        cx.notify();
    }

    /// Open the context menu for a card.
    pub fn open_menu(&mut self, id: &str, cx: &mut Context<Self>) {
        self.menu_card = Some(id.to_string());
        cx.notify();
    }

    /// Close the context menu.
    pub fn close_menu(&mut self, cx: &mut Context<Self>) {
        self.menu_card = None;
        cx.notify();
    }

    /// Open the edit dialog for a card, seeding its content.
    pub fn begin_edit(&mut self, id: &str, window: &mut Window, cx: &mut Context<Self>) {
        let Some(card) = self.cards.iter().find(|card| card.id == id).cloned() else {
            return;
        };
        let content = card.content.clone();
        self.menu_card = None;
        self.editing = Some(edit::EditState::new(card));
        let input = self.edit_input.clone();
        input.update(cx, |input, cx| {
            input.set_value(content, window, cx);
            input.focus(window, cx);
        });
        cx.notify();
    }

    /// Close the edit dialog without saving.
    pub fn cancel_edit(&mut self, cx: &mut Context<Self>) {
        self.editing = None;
        cx.notify();
    }

    /// Select the marker in the open edit dialog (`None` clears it).
    pub fn select_marker(&mut self, marker: Option<&str>, cx: &mut Context<Self>) {
        if let Some(editing) = self.editing.as_mut() {
            editing.marker = marker.map(str::to_string);
        }
        cx.notify();
    }

    /// Select the priority in the open edit dialog (`None` clears it).
    pub fn select_priority(&mut self, priority: Option<&str>, cx: &mut Context<Self>) {
        if let Some(editing) = self.editing.as_mut() {
            editing.priority = priority.map(str::to_string);
        }
        cx.notify();
    }

    /// Save the edit dialog through the provider.
    pub fn save_edit(&mut self, cx: &mut Context<Self>) {
        let Some(editing) = self.editing.clone() else {
            return;
        };
        let mut card = editing.card;
        card.content = card.content.trim().to_string();
        card.marker = editing.marker;
        card.priority = editing.priority;
        match self.provider.update_kanban_card(card) {
            Ok(()) => {
                self.editing = None;
                self.reload();
            }
            Err(error) => {
                if let Some(editing) = self.editing.as_mut() {
                    editing.error = Some(error);
                }
            }
        }
        cx.notify();
    }

    /// Delete a card through the provider.
    pub fn delete_card(&mut self, id: &str, cx: &mut Context<Self>) {
        let Some(card) = self.cards.iter().find(|card| card.id == id).cloned() else {
            return;
        };
        match self.provider.delete_kanban_card(&card.id, &card.page_path) {
            Ok(()) => {
                self.menu_card = None;
                self.editing = None;
                self.reload();
            }
            Err(error) => self.message = Some(error),
        }
        cx.notify();
    }

    /// Navigate to the page a card lives on.
    pub fn open_page(&mut self, id: &str, cx: &mut Context<Self>) {
        if let Some(card) = self.cards.iter().find(|card| card.id == id) {
            self.pending_navigation = Some(card.page_path.clone());
            cx.notify();
        }
    }

    /// Navigate to today's journal (creating it when absent).
    pub fn open_today_journal(&mut self, cx: &mut Context<Self>) {
        match self.provider.ensure_today_journal() {
            Ok(path) => self.pending_navigation = Some(path),
            Err(error) => self.message = Some(error),
        }
        cx.notify();
    }

    /// Highlight (or clear) the current drop target.
    pub fn set_drag_target(&mut self, column: Option<ColumnId>, cx: &mut Context<Self>) {
        if self.drag_target != column {
            self.drag_target = column;
            cx.notify();
        }
    }

    /// Dismiss the message bar.
    pub fn dismiss_message(&mut self, cx: &mut Context<Self>) {
        self.message = None;
        cx.notify();
    }
}

impl PanelRequests for KanbanView {
    fn pending_navigation(&self) -> Option<&str> {
        self.pending_navigation.as_deref()
    }

    fn clear_pending_navigation(&mut self) {
        self.pending_navigation = None;
    }

    fn pending_toast(&self) -> Option<&str> {
        None
    }

    fn take_pending_toast(&mut self) -> Option<String> {
        None
    }
}

impl Render for KanbanView {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let (background, foreground) = {
            let theme = cx.theme();
            (theme.colors.background, theme.colors.foreground)
        };
        div()
            .id("kanban-panel")
            .debug_selector(|| "kanban-panel".to_string())
            .flex()
            .flex_col()
            .size_full()
            .bg(background)
            .text_color(foreground)
            .child(board::header(self, cx))
            .child(board::message_bar(self, cx))
            .child(
                div()
                    .flex_1()
                    .overflow_hidden()
                    .child(board::board(self, cx)),
            )
            .when(self.editing.is_some(), |element| {
                element.child(edit::overlay(self, cx))
            })
    }
}
