//! [`PanelData`] implementation for the deterministic fixture provider.
//!
//! The fixture answers every panel call from seeded state so tests and the
//! capture harness can exercise the native panels without a vault.

use chrono::{Duration, Utc};

use stratum_app::reminders::due::parse_due;

use super::{data, kanban, panel_state, whiteboards, FixtureProvider};
use crate::provider::panels::{
    AskAnswer, AskSource, FlashcardRow, GraphPanelData, GraphSettingsRow, InstalledThemeRow,
    KanbanCardRow, LibraryItemRow, PanelData, PluginCatalogSnapshot, PluginRow, QueryTable,
    ReminderRow, SavedQueryRow, SearchHit, TemplateRow, ThemeCatalogSnapshot, ThemeValuesRow,
    WhiteboardRow,
};

impl PanelData for FixtureProvider {
    fn search_blocks(&self, query: &str, limit: usize) -> Result<Vec<SearchHit>, String> {
        Ok(data::fixture_search_hits(query, limit))
    }

    fn search_by_tag(&self, tag: &str) -> Result<Vec<SearchHit>, String> {
        Ok(data::fixture_tag_hits(tag))
    }

    fn rebuild_search_index(&self) -> Result<String, String> {
        Ok("Indexed 4 pages".to_string())
    }

    fn run_query(&self, datalog: &str) -> Result<QueryTable, String> {
        if datalog.trim().is_empty() {
            return Err("empty query".to_string());
        }
        if !datalog.contains(":find") {
            return Err("query_missing_find".to_string());
        }
        Ok(QueryTable {
            columns: vec!["?b".to_string(), "?content".to_string()],
            rows: vec![
                vec![
                    "11111111-1111-5111-8111-111111111111".to_string(),
                    "Fixture TODO row".to_string(),
                ],
                vec![
                    "22222222-2222-5222-8222-222222222222".to_string(),
                    "Project planning row".to_string(),
                ],
                vec![
                    "33333333-3333-5333-8333-333333333333".to_string(),
                    "Reading queue row".to_string(),
                ],
            ],
        })
    }

    fn saved_queries(&self) -> Result<Vec<SavedQueryRow>, String> {
        Ok(panel_state(self)?.saved_queries.clone())
    }

    fn save_query(&self, name: &str, query: &str) -> Result<SavedQueryRow, String> {
        let name = name.trim();
        if name.is_empty() {
            return Err("query name is required".to_string());
        }
        let mut state = panel_state(self)?;
        let row = SavedQueryRow {
            name: name.to_string(),
            query: query.to_string(),
            updated_at: Utc::now().to_rfc3339(),
        };
        match state
            .saved_queries
            .iter_mut()
            .find(|existing| existing.name == name)
        {
            Some(existing) => *existing = row.clone(),
            None => state.saved_queries.push(row.clone()),
        }
        Ok(row)
    }

    fn rename_query(&self, old_name: &str, new_name: &str) -> Result<SavedQueryRow, String> {
        let new_name = new_name.trim();
        if new_name.is_empty() {
            return Err("query name is required".to_string());
        }
        let mut state = panel_state(self)?;
        if new_name != old_name
            && state
                .saved_queries
                .iter()
                .any(|existing| existing.name == new_name)
        {
            return Err(format!("query already exists: {new_name}"));
        }
        let row = state
            .saved_queries
            .iter_mut()
            .find(|existing| existing.name == old_name)
            .ok_or_else(|| format!("query not found: {old_name}"))?;
        row.name = new_name.to_string();
        row.updated_at = Utc::now().to_rfc3339();
        Ok(row.clone())
    }

    fn delete_query(&self, name: &str) -> Result<(), String> {
        let mut state = panel_state(self)?;
        let before = state.saved_queries.len();
        state.saved_queries.retain(|existing| existing.name != name);
        if state.saved_queries.len() == before {
            return Err(format!("query not found: {name}"));
        }
        Ok(())
    }

    fn ask_notes(&self, question: &str) -> Result<AskAnswer, String> {
        if question.trim().is_empty() {
            return Err("question is required".to_string());
        }
        if question.to_lowercase().contains("decide") {
            Ok(AskAnswer {
                answer: "The notes suggest deciding after the sprint review.".to_string(),
                sources: vec![
                    AskSource {
                        path: "pages/fixture.md".to_string(),
                        snippet: "Decide after the sprint review.".to_string(),
                        score: 0.91,
                    },
                    AskSource {
                        path: "pages/Project Notes.md".to_string(),
                        snippet: "Decision log for the planning cycle.".to_string(),
                        score: 0.77,
                    },
                ],
                had_sources: true,
            })
        } else {
            Ok(AskAnswer {
                answer: "No matching notes were found.".to_string(),
                sources: Vec::new(),
                had_sources: false,
            })
        }
    }

    fn templates(&self) -> Result<Vec<TemplateRow>, String> {
        Ok(data::fixture_templates())
    }

    fn apply_template(
        &self,
        name: &str,
        target: &str,
        _variables: Vec<(String, String)>,
    ) -> Result<String, String> {
        if self
            .templates()?
            .iter()
            .any(|template| template.name == name)
        {
            Ok(target.to_string())
        } else {
            Err(format!("template not found: {name}"))
        }
    }

    fn install_starter_templates(&self) -> Result<Vec<String>, String> {
        Ok(vec![
            "Daily note".to_string(),
            "Meeting".to_string(),
            "Project".to_string(),
        ])
    }

    fn flashcards(&self) -> Result<Vec<FlashcardRow>, String> {
        Ok(data::fixture_flashcards())
    }

    fn review_card(&self, id: &str, quality: u8, _page_path: &str) -> Result<(), String> {
        if !self.flashcards()?.iter().any(|card| card.id == id) {
            return Err(format!("card not found: {id}"));
        }
        panel_state(self)?.reviewed.push((id.to_string(), quality));
        Ok(())
    }

    fn reminders(&self) -> Result<Vec<ReminderRow>, String> {
        Ok(panel_state(self)?.reminders.clone())
    }

    fn due_reminders(&self) -> Vec<ReminderRow> {
        if std::env::var("STRATUM_REMINDER_TOAST").ok().as_deref() != Some("1") {
            return Vec::new();
        }
        let now = Utc::now();
        self.reminders()
            .unwrap_or_default()
            .into_iter()
            .filter(|reminder| parse_due(&reminder.due).is_some_and(|due| due <= now))
            .collect()
    }

    fn snooze_reminder(&self, page_path: &str, block_id: &str, minutes: u32) -> Result<(), String> {
        let mut state = panel_state(self)?;
        let reminder = state
            .reminders
            .iter_mut()
            .find(|reminder| reminder.page_path == page_path && reminder.block_id == block_id)
            .ok_or_else(|| format!("reminder not found: {block_id}"))?;
        let due = parse_due(&reminder.due).ok_or_else(|| "invalid due".to_string())?;
        reminder.due = (due + Duration::minutes(minutes as i64)).to_rfc3339();
        Ok(())
    }

    fn clear_reminder(&self, page_path: &str, block_id: &str) -> Result<(), String> {
        let mut state = panel_state(self)?;
        state
            .reminders
            .retain(|reminder| !(reminder.page_path == page_path && reminder.block_id == block_id));
        Ok(())
    }

    fn set_reminder(
        &self,
        page_path: &str,
        block_id: &str,
        due: Option<String>,
    ) -> Result<(), String> {
        let mut state = panel_state(self)?;
        if let Some(due) = due {
            parse_due(&due).ok_or_else(|| "invalid due".to_string())?;
            let reminder = state
                .reminders
                .iter_mut()
                .find(|reminder| reminder.page_path == page_path && reminder.block_id == block_id)
                .ok_or_else(|| format!("reminder not found: {block_id}"))?;
            reminder.due = due;
        } else {
            state.reminders.retain(|reminder| {
                !(reminder.page_path == page_path && reminder.block_id == block_id)
            });
        }
        Ok(())
    }

    fn plugins(&self) -> Result<Vec<PluginRow>, String> {
        Ok(panel_state(self)?.plugins.clone())
    }

    fn set_plugin_enabled(&self, id: &str, enabled: bool) -> Result<PluginRow, String> {
        let mut state = panel_state(self)?;
        let plugin = state
            .plugins
            .iter_mut()
            .find(|plugin| plugin.id == id)
            .ok_or_else(|| format!("plugin not found: {id}"))?;
        plugin.enabled = enabled;
        plugin.status = if enabled { "ready" } else { "disabled" }.to_string();
        Ok(plugin.clone())
    }

    fn reload_plugin(&self, id: &str) -> Result<PluginRow, String> {
        let state = panel_state(self)?;
        state
            .plugins
            .iter()
            .find(|plugin| plugin.id == id)
            .cloned()
            .ok_or_else(|| format!("plugin not found: {id}"))
    }

    fn uninstall_plugin(&self, id: &str) -> Result<(), String> {
        let mut state = panel_state(self)?;
        let before = state.plugins.len();
        state.plugins.retain(|plugin| plugin.id != id);
        if state.plugins.len() == before {
            return Err(format!("plugin not found: {id}"));
        }
        Ok(())
    }

    fn install_plugin_path(&self, path: &str) -> Result<PluginRow, String> {
        let id = std::path::Path::new(path)
            .file_stem()
            .and_then(|stem| stem.to_str())
            .unwrap_or(path)
            .to_string();
        self.install_plugin_row(id)
    }

    fn install_plugin_url(
        &self,
        _url: &str,
        _sha256: Option<&str>,
        id: &str,
    ) -> Result<PluginRow, String> {
        self.install_plugin_row(id.to_string())
    }

    fn plugin_note_read(&self, path: &str) -> Result<String, String> {
        if path.is_empty() {
            return Err("note path is required".to_string());
        }
        Ok(format!("# {path}\n\nFixture note content."))
    }

    fn plugin_http_request(&self, url: &str, method: &str) -> Result<String, String> {
        Ok(format!("status=200 method={method} url={url} body=fixture"))
    }

    fn plugin_catalog(&self) -> Result<PluginCatalogSnapshot, String> {
        Ok(PluginCatalogSnapshot {
            entries: data::fixture_plugin_catalog(),
            offline: false,
            fetched_at: Some("2026-11-10T12:00:00Z".to_string()),
            error: None,
        })
    }

    fn theme_catalog(&self) -> Result<ThemeCatalogSnapshot, String> {
        Ok(ThemeCatalogSnapshot {
            entries: data::fixture_theme_catalog(),
            offline: false,
            fetched_at: Some("2026-11-10T12:00:00Z".to_string()),
            error: None,
        })
    }

    fn installed_themes(&self) -> Vec<InstalledThemeRow> {
        panel_state(self)
            .map(|state| state.installed_themes.clone())
            .unwrap_or_default()
    }

    fn install_theme(&self, id: &str) -> Result<InstalledThemeRow, String> {
        let entry = data::fixture_theme_catalog()
            .into_iter()
            .find(|entry| entry.id == id)
            .ok_or_else(|| format!("theme not found: {id}"))?;
        let mut state = panel_state(self)?;
        for theme in &mut state.installed_themes {
            theme.active = false;
        }
        let row = data::installed_theme_row(&entry.id, &entry.name);
        state.installed_themes.push(row.clone());
        Ok(row)
    }

    fn apply_theme(&self, id: &str) -> Result<ThemeValuesRow, String> {
        if id == data::BUILTIN_THEME_ID {
            return Ok(data::builtin_theme_values());
        }
        let entry = data::fixture_theme_catalog()
            .into_iter()
            .find(|entry| entry.id == id)
            .ok_or_else(|| format!("theme not found: {id}"))?;
        let mut state = panel_state(self)?;
        for theme in &mut state.installed_themes {
            theme.active = theme.id == id;
        }
        Ok(ThemeValuesRow {
            dark_mode: entry.dark_mode,
            primary_color: entry.primary_color,
            secondary_color: entry.secondary_color,
            font_size: entry.font_size,
        })
    }

    fn uninstall_theme(&self, id: &str) -> Result<(), String> {
        let mut state = panel_state(self)?;
        state.installed_themes.retain(|theme| theme.id != id);
        Ok(())
    }

    fn graph_panel_data(&self) -> Result<GraphPanelData, String> {
        Ok(data::fixture_graph_panel_data())
    }

    fn graph_settings(&self) -> Result<Option<GraphSettingsRow>, String> {
        Ok(Some(panel_state(self)?.graph_settings.clone()))
    }

    fn save_graph_settings(&self, settings: GraphSettingsRow) -> Result<(), String> {
        panel_state(self)?.graph_settings = settings;
        Ok(())
    }

    fn whiteboards(&self) -> Result<Vec<WhiteboardRow>, String> {
        whiteboards::whiteboards(self)
    }

    fn load_whiteboard(&self, name: &str) -> Result<String, String> {
        whiteboards::load_whiteboard(self, name)
    }

    fn save_whiteboard(&self, name: &str, content: &str) -> Result<(), String> {
        whiteboards::save_whiteboard(self, name, content)
    }

    fn create_whiteboard(&self, name: &str) -> Result<WhiteboardRow, String> {
        whiteboards::create_whiteboard(self, name)
    }

    fn rename_whiteboard(&self, old_name: &str, new_name: &str) -> Result<(), String> {
        whiteboards::rename_whiteboard(self, old_name, new_name)
    }

    fn delete_whiteboards(&self, names: Vec<String>) -> Result<(), String> {
        whiteboards::delete_whiteboards(self, names)
    }

    fn kanban_cards(&self) -> Result<Vec<KanbanCardRow>, String> {
        kanban::cards(self)
    }

    fn create_kanban_card(&self, content: &str, marker: &str) -> Result<KanbanCardRow, String> {
        kanban::create(self, content, marker)
    }

    fn update_kanban_card(&self, card: KanbanCardRow) -> Result<(), String> {
        kanban::update(self, card)
    }

    fn delete_kanban_card(&self, id: &str, _page_path: &str) -> Result<(), String> {
        kanban::delete(self, id)
    }

    fn whiteboard_library(&self) -> Result<Vec<LibraryItemRow>, String> {
        whiteboards::whiteboard_library(self)
    }

    fn save_whiteboard_library(&self, json: &str) -> Result<(), String> {
        whiteboards::save_whiteboard_library(self, json)
    }

    fn write_canvas_export(
        &self,
        name: &str,
        extension: &str,
        bytes: Vec<u8>,
    ) -> Result<String, String> {
        whiteboards::write_canvas_export(self, name, extension, bytes)
    }
}
