use pkm_core::fs_util::MdCollector;
use pkm_core::PkmResult;
use std::path::Path;

/// Initialize a new vault in the current directory.
pub(crate) fn cmd_init(vault: &Path) -> PkmResult<()> {
    let pkm_dir = vault.join(".pkm");
    std::fs::create_dir_all(&pkm_dir)?;
    std::fs::create_dir_all(vault.join("notes"))?;
    std::fs::create_dir_all(pkm_dir.join("history"))?;

    let config = pkm_core::Config {
        vault_path: vault.to_path_buf(),
        ..Default::default()
    };
    config
        .save(config.config_file_path())
        .map_err(|e| pkm_core::PkmError::Config(e.to_string()))?;

    // Create a welcome note
    let welcome_path = vault.join("notes/welcome.md");
    if !welcome_path.exists() {
        let content = "---\ntitle: Welcome to Stratum\ntags: [welcome, getting-started]\ncreated: "
            .to_string()
            + &chrono::Utc::now().format("%Y-%m-%d").to_string()
            + "\n---\n\n# Welcome to Stratum\n\nThis vault was just initialized. Create notes with `[[Wiki Links]]` to connect ideas.\n\n#welcome\n";
        std::fs::write(&welcome_path, content)?;
    }

    println!("✓ Initialized vault at {}", vault.display());
    println!("  Notes: {}", vault.join("notes").display());
    println!("  Cache: {}", pkm_dir.display());
    Ok(())
}

/// List all notes, optionally filtered by tag.
pub(crate) fn cmd_list(vault: &Path, tag: Option<&str>) -> PkmResult<()> {
    let notes = MdCollector::new().max_depth(8).collect(vault)?;
    let filtered: Vec<_> = if let Some(t) = tag {
        notes
            .into_iter()
            .filter(|p| {
                if let Ok(content) = std::fs::read_to_string(p) {
                    content.contains(&format!("#{}", t)) || content.contains(&format!("- {}", t))
                } else {
                    false
                }
            })
            .collect()
    } else {
        notes
    };

    if filtered.is_empty() {
        println!("No notes found.");
        return Ok(());
    }

    println!("{} notes:", filtered.len());
    for path in &filtered {
        let rel = path.strip_prefix(vault).unwrap_or(path);
        let content = std::fs::read_to_string(path).unwrap_or_default();
        let parsed = pkm_markdown::parser::parse_raw(&content);
        let title = parsed.frontmatter.title.as_deref().unwrap_or(
            rel.file_stem()
                .and_then(|s| s.to_str())
                .unwrap_or("untitled"),
        );
        println!(
            "  {:60} {}",
            rel.display(),
            if title.len() > 30 {
                format!("{}…", &title[..30])
            } else {
                title.to_string()
            }
        );
    }
    Ok(())
}

/// Show a single note with its frontmatter and body.
pub(crate) fn cmd_show(vault: &Path, path: &str) -> PkmResult<()> {
    let full_path = vault.join(path);
    if !full_path.exists() {
        eprintln!("Note not found: {}", full_path.display());
        return Ok(());
    }
    let content = std::fs::read_to_string(&full_path)?;
    let parsed = pkm_markdown::parser::parse_raw(&content);

    println!("╔══════════════════════════════════════╗");
    if let Some(title) = &parsed.frontmatter.title {
        println!("║  {}", title);
    }
    println!("║  Path: {}", path);
    if !parsed.frontmatter.tags.is_empty() {
        println!("║  Tags: {}", parsed.frontmatter.tags.join(", "));
    }
    if !parsed.links.is_empty() {
        println!(
            "║  Links: {}",
            parsed
                .links
                .iter()
                .map(|l| l.target.as_str())
                .collect::<Vec<_>>()
                .join(", ")
        );
    }
    println!("╚══════════════════════════════════════╝");
    println!("\n{}", parsed.body);
    Ok(())
}

/// Create a new note from a path and optional title.
pub(crate) fn cmd_create(vault: &Path, path: &str, title: Option<&str>) -> PkmResult<()> {
    let full_path = vault.join(path);
    if full_path.exists() {
        eprintln!("Note already exists: {}", full_path.display());
        return Ok(());
    }

    let default_title = title.unwrap_or(
        path.trim_end_matches(".md")
            .split('/')
            .next_back()
            .unwrap_or("untitled"),
    );
    let today = chrono::Utc::now().format("%Y-%m-%d").to_string();
    let content = format!(
        "---\ntitle: {}\ncreated: {}\ntags: []\n---\n\n# {}\n\n",
        default_title, today, default_title
    );

    if let Some(parent) = full_path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    std::fs::write(&full_path, &content)?;
    println!("✓ Created {}", path);
    Ok(())
}

/// Search notes for a query string.
pub(crate) fn cmd_search(vault: &Path, query: &str) -> PkmResult<()> {
    let notes = MdCollector::new().max_depth(8).collect(vault)?;
    let q = query.to_lowercase();
    let mut results = Vec::new();

    for path in notes {
        let content = std::fs::read_to_string(&path).unwrap_or_default();
        if content.to_lowercase().contains(&q) {
            let rel = path
                .strip_prefix(vault)
                .unwrap_or(&path)
                .display()
                .to_string();
            let parsed = pkm_markdown::parser::parse_raw(&content);
            let title = parsed.frontmatter.title.unwrap_or_default();
            let snippet = content
                .lines()
                .find(|l| l.to_lowercase().contains(&q))
                .unwrap_or("")
                .to_string();
            results.push((rel, title, snippet));
        }
    }

    if results.is_empty() {
        println!("No matches for '{}'", query);
        return Ok(());
    }

    println!("{} results for '{}':\n", results.len(), query);
    for (rel, title, snippet) in &results {
        println!(
            "  {} — {}",
            rel,
            if title.len() > 40 {
                format!("{}…", &title[..40])
            } else {
                title.clone()
            }
        );
        println!("    {}", snippet);
        println!();
    }
    Ok(())
}
