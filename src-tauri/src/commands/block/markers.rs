//! Block marker toggling (TODO/DOING/DONE) and marker clearing.

use crate::commands::vault::AppState;
use uuid::Uuid;

// update_block, delete_block, and insert_block operate solely on SQLite and
// do NOT write to disk or trigger IndexEngine refresh.  The next save_blocks
// call (which always follows these operations in the editing flow) will
// re-serialize the full page to disk and call refresh_page, keeping
// everything consistent.

fn extract_title_from_frontmatter(content: &str) -> Option<String> {
    let content = content.trim();
    if let Some(rest) = content.strip_prefix("---") {
        if let Some(end) = rest.find("---") {
            let frontmatter = &rest[..end];
            for line in frontmatter.lines() {
                if let Some(val) = line.strip_prefix("title:") {
                    return Some(val.trim().to_string());
                }
            }
        }
    }
    None
}
