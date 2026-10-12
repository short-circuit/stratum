use pkm_core::fs_util::MdCollector;
use pkm_core::PkmResult;
use std::path::Path;

/// Show vault statistics.
pub(crate) fn cmd_stats(vault: &Path) -> PkmResult<()> {
    let notes = MdCollector::new().max_depth(8).collect(vault)?;
    let mut total_bytes = 0u64;
    let mut total_links = 0usize;
    let mut tags = std::collections::HashSet::new();

    for path in &notes {
        if let Ok(meta) = path.metadata() {
            total_bytes += meta.len();
        }
        if let Ok(content) = std::fs::read_to_string(path) {
            let parsed = pkm_markdown::parser::parse_raw(&content);
            total_links += parsed.links.len();
            for t in parsed.tags {
                tags.insert(t.name);
            }
        }
    }

    println!("Vault Statistics");
    println!("  Notes:     {}", notes.len());
    println!("  Tags:      {}", tags.len());
    println!("  Links:     {}", total_links);
    println!("  Size:      {}", format_size(total_bytes));
    println!("  Location:  {}", vault.display());
    Ok(())
}

/// Show graph information: nodes, edges and orphaned notes.
pub(crate) fn cmd_graph(vault: &Path) -> PkmResult<()> {
    let notes = MdCollector::new().max_depth(8).collect(vault)?;
    let mut edges = Vec::new();
    let mut nodes = std::collections::HashSet::new();

    for path in &notes {
        let rel = path
            .strip_prefix(vault)
            .unwrap_or(path)
            .display()
            .to_string();
        nodes.insert(rel.clone());
        if let Ok(content) = std::fs::read_to_string(path) {
            let parsed = pkm_markdown::parser::parse_raw(&content);
            for link in &parsed.links {
                edges.push((rel.clone(), link.target.clone()));
            }
        }
    }

    if nodes.is_empty() {
        println!("No notes to graph.");
        return Ok(());
    }

    println!("Graph: {} nodes, {} edges", nodes.len(), edges.len());

    // Orphaned notes (no links in or out)
    let connected: std::collections::HashSet<String> = edges
        .iter()
        .flat_map(|(s, t)| vec![s.clone(), t.clone()])
        .collect();
    let orphaned: Vec<_> = nodes.iter().filter(|n| !connected.contains(*n)).collect();
    if !orphaned.is_empty() {
        println!("\nOrphaned notes (no connections):");
        for o in &orphaned {
            println!("  ◦ {}", o);
        }
    }

    if !edges.is_empty() {
        println!("\nEdges:");
        for (src, dst) in edges.iter().take(20) {
            println!("  {}  →  {}", src, dst);
        }
        if edges.len() > 20 {
            println!("  … and {} more", edges.len() - 20);
        }
    }
    Ok(())
}

/// Show a tag cloud ordered by descending frequency.
pub(crate) fn cmd_tags(vault: &Path) -> PkmResult<()> {
    let notes = MdCollector::new().max_depth(8).collect(vault)?;
    let mut counts: std::collections::HashMap<String, usize> = std::collections::HashMap::new();

    for path in &notes {
        if let Ok(content) = std::fs::read_to_string(path) {
            let parsed = pkm_markdown::parser::parse_raw(&content);
            for tag in &parsed.tags {
                *counts.entry(tag.name.clone()).or_default() += 1;
            }
        }
    }

    if counts.is_empty() {
        println!("No tags found.");
        return Ok(());
    }

    let mut sorted: Vec<_> = counts.into_iter().collect();
    sorted.sort_by_key(|a| std::cmp::Reverse(a.1));

    println!("Tag Cloud ({} tags):\n", sorted.len());
    let max_count = sorted.first().map(|(_, c)| *c).unwrap_or(1) as f64;
    for (name, count) in &sorted {
        let bar_len = ((*count as f64 / max_count) * 40.0) as usize;
        let bar = "█".repeat(bar_len);
        println!("  {:20} {:4} {}", name, count, bar);
    }
    Ok(())
}

fn format_size(bytes: u64) -> String {
    const UNITS: &[&str] = &["B", "KB", "MB", "GB"];
    let mut size = bytes as f64;
    let mut unit_idx = 0;
    while size > 1024.0 && unit_idx < UNITS.len() - 1 {
        size /= 1024.0;
        unit_idx += 1;
    }
    format!("{:.1} {}", size, UNITS[unit_idx])
}
