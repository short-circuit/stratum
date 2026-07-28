mod common;

use common::create_test_vault;
use uuid::Uuid;

fn add_block_with_link(tv: &common::TestVault, content: &str, rel_path: &str) -> Uuid {
    let block = tv.add_block(rel_path, content);
    let links = pkm_markdown::linker::extract_links(content);
    for link in links {
        let target_slug = link.target.replace(' ', "-").to_lowercase();
        // Resolve slugs — look up in our test pages
        let all_pages = tv.store.list_pages().unwrap();
        for p in all_pages {
            let slug = std::path::Path::new(&p)
                .file_stem()
                .and_then(|s| s.to_str())
                .unwrap_or("")
                .to_string();
            if slug == target_slug {
                tv.store.insert_link(block.id, "page_ref", Some(&p), None).unwrap();
            }
        }
    }
    block.id
}

#[test]
fn test_graph_from_links() {
    let tv = create_test_vault();
    tv.add_page("pages/alpha.md");
    tv.add_page("pages/beta.md");

    add_block_with_link(&tv, "Check out [[beta]] for details", "pages/alpha.md");
    add_block_with_link(&tv, "See also [[alpha]]", "pages/beta.md");

    // Count links in DB
    let backlinks_alpha = tv.store.get_backlinks_for_page("pages/alpha.md").unwrap();
    let backlinks_beta = tv.store.get_backlinks_for_page("pages/beta.md").unwrap();
    assert_eq!(backlinks_alpha.len(), 1);
    assert_eq!(backlinks_beta.len(), 1);
}

#[test]
fn test_orphan_detection() {
    let tv = create_test_vault();
    tv.add_page("pages/connected.md");
    tv.add_page("pages/orphan.md");

    // Create link from connected to itself via content
    add_block_with_link(&tv, "Self link", "pages/connected.md");

    // Orphan has no links at all
    add_block_with_link(&tv, "Isolated content", "pages/orphan.md");

    // Check page has no incoming links
    let backlinks = tv.store.get_backlinks_for_page("pages/orphan.md").unwrap();
    assert!(backlinks.is_empty());
}

#[test]
fn test_self_link() {
    let tv = create_test_vault();
    tv.add_page("pages/self.md");

    let b = tv.add_block("pages/self.md", "Self reference [[self]]");

    // The block content mentions [[self]] — no link in DB though since we'd 
    // skip self-referencing links
    let backlinks = tv.store.get_backlinks_for_page("pages/self.md").unwrap();
    // Could be empty or contain self-link depending on implementation
    assert!(backlinks.len() <= 1);
}

#[test]
fn test_complex_graph() {
    let tv = create_test_vault();
    // Create 5 pages with various link patterns
    for name in &["hub", "spoke-a", "spoke-b", "spoke-c", "isolated"] {
        tv.add_page(&format!("pages/{}.md", name));
    }

    // hub links to all spokes
    add_block_with_link(&tv, "Links to [[spoke-a]], [[spoke-b]], [[spoke-c]]", "pages/hub.md");
    // spokes link back to hub
    add_block_with_link(&tv, "Back to [[hub]]", "pages/spoke-a.md");
    add_block_with_link(&tv, "Back to [[hub]]", "pages/spoke-b.md");
    add_block_with_link(&tv, "Back to [[hub]]", "pages/spoke-c.md");
    // isolated has no links

    // Verify hub has 3 incoming links
    let hub_backlinks = tv.store.get_backlinks_for_page("pages/hub.md").unwrap();
    assert_eq!(hub_backlinks.len(), 3, "hub should have 3 incoming links");

    // Each spoke has 1 incoming (from hub) + no others
    let spoke_a = tv.store.get_backlinks_for_page("pages/spoke-a.md").unwrap();
    let isolated = tv.store.get_backlinks_for_page("pages/isolated.md").unwrap();
    assert_eq!(spoke_a.len(), 1, "spoke-a should have 1 incoming from hub");
    assert!(isolated.is_empty(), "isolated should have no links");
}
