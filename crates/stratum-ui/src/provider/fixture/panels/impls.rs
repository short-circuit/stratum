//! [`PanelData`] implementation for the deterministic fixture provider.
//!
//! The fixture answers every panel call from seeded state so tests and the
//! capture harness can exercise the native panels without a vault.

use chrono::{Duration, Utc};

use stratum_app::reminders::due::parse_due;
use stratum_app::template::template_variables;

use super::{data, panel_state, FixtureProvider};
use crate::provider::panels::{
    AskAnswer, AskSource, FlashcardRow, InstalledThemeRow, PanelData, PluginCatalogSnapshot,
    PluginRow, QueryTable, ReminderRow, SavedQueryRow, SearchHit, TemplateRow,
    ThemeCatalogSnapshot, ThemeValuesRow,
};

impl PanelData for FixtureProvider {
    fn search_blocks(&self, query: &str, limit: usize) -> Result<Vec<SearchHit>, String> {
        if query.trim().is_empty() {
            return Ok(Vec::new());
        }
        let hits = vec![
            SearchHit {
                block_id: "fixture-search-1".to_string(),
                page_path: "pages/fixture.md".to_string(),
                snippet: format!("Fixture note mentions {query} in its opening line."),
                content: format!("Fixture note mentions {query} in its opening line."),
                score: 1.0,
            },
            SearchHit {
                block_id: "fixture-search-2".to_string(),
                page_path: "pages/Project Notes.md".to_string(),
                snippet: format!("Project planning notes reference {query}."),
                content: format!("Project planning notes reference {query}."),
                score: 0.72,
            },
            SearchHit {
                block_id: "fixture-search-3".to_string(),
                page_path: "pages/Welcome.md".to_string(),
                snippet: format!("Welcome tour points at {query}."),
                content: format!("Welcome tour points at {query}."),
                score: 0.41,
            },
        ];
        Ok(hits.into_iter().take(limit).collect())
    }

    fn search_by_tag(&self, tag: &str) -> Result<Vec<SearchHit>, String> {
        let pages: &[(&str, &str)] = match tag.trim().to_lowercase().as_str() {
            "project" => &[
                (
                    "pages/fixture.md",
                    "Project planning block tagged #project.",
                ),
                ("pages/Project Notes.md", "Sprint notes tagged #project."),
            ],
            "reading" => &[
                ("pages/Reading List.md", "Reading queue tagged #reading."),
                ("pages/Welcome.md", "Welcome tour tagged #reading."),
            ],
            _ => &[],
        };
        Ok(pages
            .iter()
            .enumerate()
            .map(|(index, (path, snippet))| SearchHit {
                block_id: format!("fixture-tag-{index}"),
                page_path: path.to_string(),
                snippet: snippet.to_string(),
                content: snippet.to_string(),
                score: 1.0,
            })
            .collect())
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
        Ok(vec![
            fixture_template(
                "Daily note",
                "Daily journal scaffold.",
                "# {{title}}\n\n## {{date}}\n\n- Morning review\n",
            ),
            fixture_template(
                "Meeting",
                "Meeting notes with attendees.",
                "# {{title}}\n\nDate: {{date}}\n\nAttendees: {{attendees}}\n",
            ),
            fixture_template(
                "Project",
                "Project brief with a date.",
                "# {{title}}\n\nStarted: {{date}}\n\n## Goal\n",
            ),
        ])
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
        Ok(vec![
            FlashcardRow {
                id: "card-1".to_string(),
                front: "What is the fixture page?".to_string(),
                back: "A deterministic page used by tests and captures.".to_string(),
                page_path: "pages/fixture.md".to_string(),
                ease_factor: 2.5,
                interval_days: 1,
                repetitions: 1,
                next_review: "2026-11-11".to_string(),
            },
            FlashcardRow {
                id: "card-2".to_string(),
                front: "What does the reading list track?".to_string(),
                back: "Offline knowledge tooling.".to_string(),
                page_path: "pages/Reading List.md".to_string(),
                ease_factor: 2.4,
                interval_days: 6,
                repetitions: 2,
                next_review: "2026-11-16".to_string(),
            },
            FlashcardRow {
                id: "card-3".to_string(),
                front: "What is the project planning cadence?".to_string(),
                back: "A sprint review every two weeks.".to_string(),
                page_path: "pages/Project Notes.md".to_string(),
                ease_factor: 2.3,
                interval_days: 3,
                repetitions: 1,
                next_review: "2026-11-13".to_string(),
            },
        ])
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
}

/// Build one deterministic fixture template row from its content.
fn fixture_template(name: &str, description: &str, content: &str) -> TemplateRow {
    TemplateRow {
        name: name.to_string(),
        path: format!("templates/{name}.md"),
        description: Some(description.to_string()),
        variables: template_variables(content),
        content: content.to_string(),
    }
}
