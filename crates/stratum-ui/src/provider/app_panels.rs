//! `PanelData` implementation for the service-backed provider.
//!
//! Every method delegates to [`StratumService`] on the provider's Tokio
//! runtime and maps the service DTOs into the panel contract. Keeping the
//! mapping here leaves `provider/mod.rs` and `provider/app.rs` under the
//! repository file-length limit.

mod kanban;
mod mappers;
mod whiteboards;

use chrono::Utc;
use stratum_app::reminders::due::parse_due;
use stratum_app::settings::GraphSettingsDto;
use stratum_app::template::template_variables;

use super::panels::{
    AskAnswer, AskSource, FlashcardRow, GraphComponentRow, GraphOrphanRow, GraphPanelData,
    GraphSettingsRow, InstalledThemeRow, KanbanCardRow, LibraryItemRow, PanelData,
    PluginCatalogSnapshot, PluginRow, QueryTable, ReminderRow, SavedQueryRow, SearchHit,
    TemplateRow, ThemeCatalogSnapshot, ThemeValuesRow, WhiteboardRow, PANEL_DATA_NOT_AVAILABLE,
};
use super::AppProvider;
use mappers::{graph_edge_row, graph_node_row, graph_settings_row, plugin_row};

/// Map a service search result into a panel hit.
fn search_hit(result: stratum_app::search::SearchResultDto) -> SearchHit {
    SearchHit {
        block_id: result.block_id,
        page_path: result.page_path,
        snippet: result.snippet,
        content: result.content,
        score: result.score,
    }
}

impl PanelData for AppProvider {
    fn search_blocks(&self, query: &str, limit: usize) -> Result<Vec<SearchHit>, String> {
        let results = self
            .runtime
            .block_on(self.service.search_blocks(query.to_string(), Some(limit)))?;
        Ok(results.results.into_iter().map(search_hit).collect())
    }

    fn search_by_tag(&self, tag: &str) -> Result<Vec<SearchHit>, String> {
        let results = self
            .runtime
            .block_on(self.service.search_by_tag(tag.to_string()))?;
        Ok(results.results.into_iter().map(search_hit).collect())
    }

    fn rebuild_search_index(&self) -> Result<String, String> {
        self.runtime
            .block_on(self.service.rebuild_search_index(&stratum_app::NoopSink))
    }

    fn run_query(&self, datalog: &str) -> Result<QueryTable, String> {
        let result = self
            .runtime
            .block_on(self.service.run_query(datalog.to_string()))?;
        Ok(QueryTable {
            columns: result.columns,
            rows: result.rows,
        })
    }

    fn saved_queries(&self) -> Result<Vec<SavedQueryRow>, String> {
        let saved = self.runtime.block_on(self.service.list_saved_queries())?;
        Ok(saved
            .into_iter()
            .map(|query| SavedQueryRow {
                name: query.name,
                query: query.query,
                updated_at: query.updated_at,
            })
            .collect())
    }

    fn save_query(&self, name: &str, query: &str) -> Result<SavedQueryRow, String> {
        let saved = self.runtime.block_on(
            self.service
                .save_saved_query(name.to_string(), query.to_string()),
        )?;
        Ok(SavedQueryRow {
            name: saved.name,
            query: saved.query,
            updated_at: saved.updated_at,
        })
    }

    fn rename_query(&self, old_name: &str, new_name: &str) -> Result<SavedQueryRow, String> {
        let saved = self.runtime.block_on(
            self.service
                .rename_saved_query(old_name.to_string(), new_name.to_string()),
        )?;
        Ok(SavedQueryRow {
            name: saved.name,
            query: saved.query,
            updated_at: saved.updated_at,
        })
    }

    fn delete_query(&self, name: &str) -> Result<(), String> {
        self.runtime
            .block_on(self.service.delete_saved_query(name.to_string()))
    }

    fn ask_notes(&self, question: &str) -> Result<AskAnswer, String> {
        let result = self
            .runtime
            .block_on(self.service.ai_rag_query(question.to_string()))?;
        Ok(AskAnswer {
            answer: result.answer,
            sources: result
                .citations
                .into_iter()
                .map(|citation| AskSource {
                    path: citation.path,
                    snippet: citation.snippet,
                    score: citation.score,
                })
                .collect(),
            had_sources: result.had_sources,
        })
    }

    fn templates(&self) -> Result<Vec<TemplateRow>, String> {
        let templates = self.runtime.block_on(self.service.list_templates())?;
        Ok(templates
            .into_iter()
            .map(|template| TemplateRow {
                variables: template_variables(&template.content),
                name: template.name,
                path: template.path,
                description: template.description,
                content: template.content,
            })
            .collect())
    }

    fn apply_template(
        &self,
        name: &str,
        target: &str,
        variables: Vec<(String, String)>,
    ) -> Result<String, String> {
        self.runtime.block_on(self.service.apply_template(
            name.to_string(),
            target.to_string(),
            variables,
        ))
    }

    fn install_starter_templates(&self) -> Result<Vec<String>, String> {
        self.runtime
            .block_on(self.service.install_starter_templates(None))
    }

    fn flashcards(&self) -> Result<Vec<FlashcardRow>, String> {
        let cards = self.runtime.block_on(self.service.generate_flashcards())?;
        Ok(cards
            .into_iter()
            .map(|card| FlashcardRow {
                id: card.id,
                front: card.front,
                back: card.back,
                page_path: card.page_path,
                ease_factor: card.ease_factor,
                interval_days: card.interval_days,
                repetitions: card.repetitions,
                next_review: card.next_review,
            })
            .collect())
    }

    fn review_card(&self, id: &str, quality: u8, page_path: &str) -> Result<(), String> {
        self.runtime
            .block_on(
                self.service
                    .review_card(id.to_string(), quality, page_path.to_string()),
            )
            .map(|_| ())
    }

    fn reminders(&self) -> Result<Vec<ReminderRow>, String> {
        let reminders = self.runtime.block_on(self.service.list_reminders())?;
        Ok(reminders
            .into_iter()
            .map(|reminder| ReminderRow {
                block_id: reminder.block_id,
                page_path: reminder.page_path,
                page_title: reminder.page_title,
                content: reminder.content,
                due: reminder.due,
            })
            .collect())
    }

    fn due_reminders(&self) -> Vec<ReminderRow> {
        let now = Utc::now();
        self.reminders()
            .unwrap_or_default()
            .into_iter()
            .filter(|reminder| parse_due(&reminder.due).is_some_and(|due| due <= now))
            .collect()
    }

    fn snooze_reminder(&self, page_path: &str, block_id: &str, minutes: u32) -> Result<(), String> {
        self.runtime.block_on(self.service.snooze_reminder(
            page_path.to_string(),
            block_id.to_string(),
            minutes,
        ))
    }

    fn clear_reminder(&self, page_path: &str, block_id: &str) -> Result<(), String> {
        self.runtime.block_on(
            self.service
                .clear_reminder(page_path.to_string(), block_id.to_string()),
        )
    }

    fn set_reminder(
        &self,
        page_path: &str,
        block_id: &str,
        due: Option<String>,
    ) -> Result<(), String> {
        self.runtime.block_on(self.service.set_block_reminder(
            page_path.to_string(),
            block_id.to_string(),
            due,
        ))
    }

    fn plugins(&self) -> Result<Vec<PluginRow>, String> {
        let list = self.runtime.block_on(self.service.plugins_list())?;
        Ok(list.plugins.into_iter().map(plugin_row).collect())
    }

    fn set_plugin_enabled(&self, id: &str, enabled: bool) -> Result<PluginRow, String> {
        let info = if enabled {
            self.runtime
                .block_on(self.service.plugins_enable(id.to_string()))?
        } else {
            self.runtime
                .block_on(self.service.plugins_disable(id.to_string()))?
        };
        Ok(plugin_row(info))
    }

    fn reload_plugin(&self, id: &str) -> Result<PluginRow, String> {
        let info = self
            .runtime
            .block_on(self.service.plugins_reload(id.to_string()))?;
        Ok(plugin_row(info))
    }

    fn uninstall_plugin(&self, id: &str) -> Result<(), String> {
        self.runtime
            .block_on(self.service.plugins_uninstall(id.to_string()))
            .map(|_| ())
    }

    fn install_plugin_path(&self, path: &str) -> Result<PluginRow, String> {
        let info = self
            .runtime
            .block_on(self.service.plugins_install(path.to_string()))?;
        Ok(plugin_row(info))
    }

    fn install_plugin_url(
        &self,
        url: &str,
        sha256: Option<&str>,
        id: &str,
    ) -> Result<PluginRow, String> {
        let info = self.runtime.block_on(self.service.plugins_install_url(
            url.to_string(),
            sha256.map(str::to_string),
            id.to_string(),
        ))?;
        Ok(plugin_row(info))
    }

    fn plugin_note_read(&self, path: &str) -> Result<String, String> {
        let note = self
            .runtime
            .block_on(self.service.plugin_note_read(path.to_string()))?;
        Ok(note.content)
    }

    fn plugin_http_request(&self, url: &str, method: &str) -> Result<String, String> {
        let response = self.runtime.block_on(self.service.plugin_http_request(
            Some(method.to_string()),
            url.to_string(),
            None,
            None,
            None,
        ))?;
        Ok(format!("status={} body={}", response.status, response.body))
    }

    fn plugin_catalog(&self) -> Result<PluginCatalogSnapshot, String> {
        let snapshot = self.runtime.block_on(self.service.plugins_catalog())?;
        Ok(PluginCatalogSnapshot {
            entries: snapshot.entries,
            offline: snapshot.offline,
            fetched_at: snapshot.fetched_at,
            error: snapshot.error,
        })
    }

    fn theme_catalog(&self) -> Result<ThemeCatalogSnapshot, String> {
        let snapshot = self.runtime.block_on(self.service.themes_catalog())?;
        Ok(ThemeCatalogSnapshot {
            entries: snapshot.entries,
            offline: snapshot.offline,
            fetched_at: snapshot.fetched_at,
            error: snapshot.error,
        })
    }

    fn installed_themes(&self) -> Vec<InstalledThemeRow> {
        self.runtime
            .block_on(self.service.list_installed_themes())
            .unwrap_or_default()
            .into_iter()
            .map(|theme| InstalledThemeRow {
                id: theme.id,
                name: theme.name,
                active: theme.active,
            })
            .collect()
    }

    fn install_theme(&self, id: &str) -> Result<InstalledThemeRow, String> {
        let catalog = self
            .runtime
            .block_on(self.service.themes_catalog())
            .map_err(|error| format!("{PANEL_DATA_NOT_AVAILABLE}: {error}"))?;
        let entry = catalog
            .entries
            .into_iter()
            .find(|entry| entry.id == id)
            .ok_or_else(|| format!("theme_not_found: {id}"))?;
        let installed = self.runtime.block_on(self.service.install_theme(entry))?;
        Ok(InstalledThemeRow {
            id: installed.id,
            name: installed.name,
            active: installed.active,
        })
    }

    fn apply_theme(&self, id: &str) -> Result<ThemeValuesRow, String> {
        let values = self
            .runtime
            .block_on(self.service.apply_theme(id.to_string()))?;
        Ok(ThemeValuesRow {
            dark_mode: values.dark_mode,
            primary_color: values.primary_color,
            secondary_color: values.secondary_color,
            font_size: values.font_size,
        })
    }

    fn uninstall_theme(&self, id: &str) -> Result<(), String> {
        self.runtime
            .block_on(self.service.uninstall_theme(id.to_string()))
    }

    fn graph_panel_data(&self) -> Result<GraphPanelData, String> {
        let panel = self.runtime.block_on(self.service.get_graph_panel_data())?;
        Ok(GraphPanelData {
            vault_path: panel.graph.vault_path.clone(),
            nodes: panel.graph.nodes.iter().map(graph_node_row).collect(),
            edges: panel.graph.edges.iter().map(graph_edge_row).collect(),
            components: panel
                .components
                .iter()
                .map(|component| GraphComponentRow {
                    nodes: component.nodes.iter().map(graph_node_row).collect(),
                    size: component.size,
                })
                .collect(),
            orphans: panel
                .orphans
                .iter()
                .map(|orphan| GraphOrphanRow {
                    slug: orphan.slug.clone(),
                    title: orphan.title.clone(),
                    path: orphan.path.clone(),
                })
                .collect(),
        })
    }

    fn graph_settings(&self) -> Result<Option<GraphSettingsRow>, String> {
        let settings = self.runtime.block_on(self.service.get_settings())?;
        Ok(Some(graph_settings_row(settings.graph)))
    }

    fn save_graph_settings(&self, settings: GraphSettingsRow) -> Result<(), String> {
        self.runtime
            .block_on(self.service.save_graph_settings(GraphSettingsDto {
                show_connected: settings.show_connected,
                show_orphaned: settings.show_orphaned,
                show_tags: settings.show_tags,
                charge_strength: settings.charge_strength,
                link_distance: settings.link_distance,
                alpha_decay: settings.alpha_decay,
                velocity_decay: settings.velocity_decay,
                link_curvature: settings.link_curvature,
                use_3d: settings.use_3d,
            }))
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

    fn kanban_cards(&self) -> Result<Vec<KanbanCardRow>, String> {
        kanban::cards(self)
    }

    fn create_kanban_card(&self, content: &str, marker: &str) -> Result<KanbanCardRow, String> {
        kanban::create(self, content, marker)
    }

    fn update_kanban_card(&self, card: KanbanCardRow) -> Result<(), String> {
        kanban::update(self, card)
    }

    fn delete_kanban_card(&self, id: &str, page_path: &str) -> Result<(), String> {
        kanban::delete(self, id, page_path)
    }
}
