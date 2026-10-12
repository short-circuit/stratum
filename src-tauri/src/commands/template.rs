//! Template management commands.
//!
//! Templates are markdown pages stored in `templates/` directory with
//! variable substitution support using `{{var}}` syntax.

use crate::commands::vault::AppState;
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct TemplateDto {
    pub name: String,
    pub path: String,
    pub content: String,
    pub description: Option<String>,
}

#[tauri::command]
pub async fn list_templates(state: tauri::State<'_, AppState>) -> Result<Vec<TemplateDto>, String> {
    let state = state.lock().map_err(|e| e.to_string())?;
    let templates_dir = state.vault_path.join("templates");
    if !templates_dir.exists() {
        return Ok(Vec::new());
    }

    let mut templates = Vec::new();
    let entries = std::fs::read_dir(&templates_dir).map_err(|e| e.to_string())?;

    for entry in entries {
        let entry = entry.map_err(|e| e.to_string())?;
        let path = entry.path();
        if path.extension().and_then(|e| e.to_str()) == Some("md") {
            let content = std::fs::read_to_string(&path).map_err(|e| e.to_string())?;
            let name = path
                .file_stem()
                .and_then(|s| s.to_str())
                .unwrap_or("untitled")
                .to_string();
            let rel_path = path
                .strip_prefix(&state.vault_path)
                .unwrap_or(&path)
                .to_string_lossy()
                .to_string();

            // Extract description from YAML frontmatter
            let (fm, _) = pkm_markdown::parser::parse_frontmatter(&content);
            let description = fm.extra.get("description").and_then(|v| {
                if let serde_yaml::Value::String(s) = v {
                    Some(s.clone())
                } else {
                    None
                }
            });

            templates.push(TemplateDto {
                name,
                path: rel_path,
                content,
                description,
            });
        }
    }

    Ok(templates)
}

#[tauri::command]
pub async fn save_template(
    name: String,
    content: String,
    state: tauri::State<'_, AppState>,
) -> Result<(), String> {
    let state = state.lock().map_err(|e| e.to_string())?;
    let templates_dir = state.vault_path.join("templates");
    std::fs::create_dir_all(&templates_dir).map_err(|e| e.to_string())?;

    let path = templates_dir.join(format!("{}.md", name));
    std::fs::write(&path, &content).map_err(|e| e.to_string())?;
    Ok(())
}

#[tauri::command]
pub async fn apply_template(
    template_name: String,
    target_page: String,
    variables: Vec<(String, String)>,
    state: tauri::State<'_, AppState>,
) -> Result<String, String> {
    let state = state.lock().map_err(|e| e.to_string())?;
    let template_path = state
        .vault_path
        .join("templates")
        .join(format!("{}.md", template_name));

    if !template_path.exists() {
        return Err(format!("Template not found: {}", template_name));
    }

    let content = std::fs::read_to_string(&template_path).map_err(|e| e.to_string())?;

    // Substitute variables: {{var}} → value
    let mut result = content;
    for (key, value) in &variables {
        let pattern = format!("{{{{{}}}}}", key);
        result = result.replace(&pattern, value);
    }

    // Also substitute built-in variables
    let now = chrono::Utc::now();
    result = result.replace("{{date}}", &now.format("%Y-%m-%d").to_string());
    result = result.replace("{{time}}", &now.format("%H:%M:%S").to_string());
    result = result.replace("{{datetime}}", &now.format("%Y-%m-%d %H:%M:%S").to_string());
    result = result.replace("{{title}}", &target_page);

    // Write to target page
    let target_path = state.vault_path.join(&target_page);
    if let Some(parent) = target_path.parent() {
        std::fs::create_dir_all(parent).map_err(|e| e.to_string())?;
    }
    std::fs::write(&target_path, &result).map_err(|e| e.to_string())?;

    Ok(result)
}

/// Install bundled starter templates into `<vault>/templates/`.
///
/// When `names` is `None`, every bundled starter template is installed;
/// otherwise only the named subset. Installation is idempotent: a file that
/// already exists is never overwritten (so user edits are preserved), but its
/// name is still included in the returned list so the UI can report it as
/// already present. The `templates/` directory is created if missing.
///
/// Returns the sorted, de-duplicated list of template names present in the
/// templates directory after the call.
#[tauri::command]
pub async fn install_starter_templates(
    names: Option<Vec<String>>,
    state: tauri::State<'_, AppState>,
) -> Result<Vec<String>, String> {
    let state = state.lock().map_err(|e| e.to_string())?;
    let templates_dir = state.vault_path.join("templates");
    std::fs::create_dir_all(&templates_dir).map_err(|e| e.to_string())?;

    let selected = names.unwrap_or_else(|| {
        pkm_core::starter_template_names()
            .into_iter()
            .map(|s| s.to_string())
            .collect()
    });

    let mut present: Vec<String> = Vec::new();
    for name in &selected {
        let template = pkm_core::starter_templates()
            .iter()
            .find(|t| t.name == name);
        if let Some(t) = template {
            let path = templates_dir.join(format!("{}.md", name));
            if !path.exists() {
                std::fs::write(&path, t.content).map_err(|e| e.to_string())?;
            }
            present.push(name.clone());
        }
    }

    present.sort();
    present.dedup();
    Ok(present)
}

/// Seed a sample vault (a small set of linked demo notes) when empty.
///
/// Writes plain Markdown pages under `<vault>/pages/` — a welcome note plus
/// linked project notes — demonstrating wiki-links, backlinks, and the graph.
/// The notes are created ONLY when the vault has no pages yet; otherwise the
/// command is a no-op returning an empty list (never clobbering existing work).
///
/// Returns the list of created vault-relative paths (empty when skipped).
#[tauri::command]
pub async fn install_starter_sample(
    state: tauri::State<'_, AppState>,
) -> Result<Vec<String>, String> {
    let state = state.lock().map_err(|e| e.to_string())?;

    // Only seed when the vault is empty: prefer the block store's page count,
    // falling back to scanning the `pages/` directory if the store is
    // unavailable (e.g. SQLite not yet initialized).
    let empty = match state.get_store() {
        Ok(store) => match store.page_count() {
            Ok(n) => n == 0,
            Err(_) => false,
        },
        Err(_) => {
            let pages_dir = state.vault_path.join("pages");
            match std::fs::read_dir(&pages_dir) {
                Ok(rd) => {
                    rd.filter_map(|e| e.ok())
                        .filter(|e| e.path().extension().and_then(|x| x.to_str()) == Some("md"))
                        .count()
                        == 0
                }
                Err(_) => true,
            }
        }
    };

    if !empty {
        return Ok(Vec::new());
    }

    let notes: [(&str, &str); 4] = [
        (
            "pages/Getting Started.md",
            "---\ntitle: Getting Started\ntags: [welcome]\n---\n\n# Getting Started\n\nWelcome to Stratum. This sample vault shows how wiki-links, backlinks, and the knowledge graph work.\n\n## Explore\n\n- [ ] TODO: Open [[Project Alpha]]\n- [ ] TODO: Open [[Second Project]]\n- [ ] TODO: Add a note to [[Quick Notes]]\n\n- #welcome\n",
        ),
        (
            "pages/Project Alpha.md",
            "---\ntitle: Project Alpha\ntags: [project]\n---\n\n# Project Alpha\n\nA sample project note linked to [[Getting Started]] and [[Second Project]].\n\n## Status\n\n- [ ] TODO: Define scope\n- [ ] DOING: Build the prototype\n- [ ] DONE: Research\n\n- #project\n",
        ),
        (
            "pages/Second Project.md",
            "---\ntitle: Second Project\ntags: [project, demo]\n---\n\n# Second Project\n\nA second project note that links back to [[Project Alpha]].\n\n## Notes\n\n- Track progress in [[Project Alpha]].\n\n- #project\n- #demo\n",
        ),
        (
            "pages/Quick Notes.md",
            "---\ntitle: Quick Notes\ntags: [notes]\n---\n\n# Quick Notes\n\nA scratch note for quick captures.\n\n## Ideas\n\n- Links back to [[Project Alpha]].\n\n- #notes\n",
        ),
    ];

    let mut created = Vec::new();
    for (rel, content) in notes {
        let path = state.vault_path.join(rel);
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent).map_err(|e| e.to_string())?;
        }
        std::fs::write(&path, content).map_err(|e| e.to_string())?;
        created.push(rel.to_string());
    }

    Ok(created)
}
