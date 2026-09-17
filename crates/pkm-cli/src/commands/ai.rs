use pkm_core::PkmResult;
use std::path::Path;

/// AI chat (mock provider guidance).
pub(crate) fn cmd_ask(_vault: &Path, question: &str) -> PkmResult<()> {
    println!("🤖 AI Chat");
    println!("   Q: {}", question);
    println!("   A: To use AI features, configure a provider in settings.toml");
    println!("      and run with a running Ollama/OpenAI-compatible endpoint.");
    println!();
    println!("   Mock response: You asked about '{}'.", question);
    println!("   This would be answered by the RAG pipeline using your notes.");
    Ok(())
}

/// Rebuild the vault search index from `.md` files on disk.
///
/// This is required before a RAG query can retrieve note chunks (the index
/// is what `RagEngine` searches). The desktop app maintains this index
/// continuously; the CLI only does so when `--index` is passed to avoid a
/// full re-index on every query.
pub(crate) fn cmd_index(vault: &Path) -> PkmResult<()> {
    let mut engine = crate::IndexEngine::new(vault)?;
    let notes = engine.rebuild_all(None)?;
    engine.flush()?;
    println!("✓ Indexed {} notes in {}", notes.len(), vault.display());
    Ok(())
}
