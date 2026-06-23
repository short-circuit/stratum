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
