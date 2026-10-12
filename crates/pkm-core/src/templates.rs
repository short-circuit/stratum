//! Bundled starter templates for the first-run onboarding flow.
//!
//! This module ships a curated set of Markdown templates that give a brand-new
//! user a working, useful vault immediately. Each template is a plain `.md`
//! string with YAML frontmatter, wiki-links (`[[...]]`), and tags (`#tag`).
//!
//! Templates are installed idempotently into a vault's `templates/` directory
//! by the Tauri `install_starter_templates` command (see `src-tauri`). The
//! content uses ONLY the variable placeholders the existing `apply_template`
//! command supports (`{{date}}`, `{{time}}`, `{{datetime}}`, `{{title}}`) plus
//! arbitrary user variables (e.g. `{{author}}`), so a template can be applied
//! again with values substituted. Where no user variable is filled, the literal
//! `{{var}}` text remains — acceptable because templates are directly editable
//! files on disk.
//!
//! `pkm-core` is intentionally standalone (it does not depend on `pkm-markdown`)
//! per the crate dependency graph, so the unit tests below assert template
//! structure by inspection (frontmatter marker, heading, wiki-link, tag) rather
//! than invoking a parser.

/// A single bundled starter template.
///
/// `name` is the file stem (no extension, no path separators) used as the
/// `templates/<name>.md` filename; `content` is the full Markdown body,
/// including YAML frontmatter.
pub struct StarterTemplate {
    /// File stem of the template (e.g. "daily-note" → `templates/daily-note.md`).
    pub name: &'static str,
    /// Full Markdown content, starting with `---\n` frontmatter.
    pub content: &'static str,
}

/// Return the 12 bundled starter templates.
///
/// Order is stable and intentional: a "getting started" friendly sequence that
/// starts with the highest-value templates (daily note, meeting notes) and ends
/// with a demo graph that exercises backlinks and the knowledge graph.
pub fn starter_templates() -> &'static [StarterTemplate] {
    &[
        StarterTemplate {
            name: "daily-note",
            content: DAILY_NOTE,
        },
        StarterTemplate {
            name: "meeting-notes",
            content: MEETING_NOTES,
        },
        StarterTemplate {
            name: "reading-list",
            content: READING_LIST,
        },
        StarterTemplate {
            name: "book-notes",
            content: BOOK_NOTES,
        },
        StarterTemplate {
            name: "project-tracker",
            content: PROJECT_TRACKER,
        },
        StarterTemplate {
            name: "research-notes",
            content: RESEARCH_NOTES,
        },
        StarterTemplate {
            name: "flashcards-starter",
            content: FLASHCARDS_STARTER,
        },
        StarterTemplate {
            name: "kanban-starter",
            content: KANBAN_STARTER,
        },
        StarterTemplate {
            name: "whiteboard-starter",
            content: WHITEBOARD_STARTER,
        },
        StarterTemplate {
            name: "habit-tracker",
            content: HABIT_TRACKER,
        },
        StarterTemplate {
            name: "quick-notes",
            content: QUICK_NOTES,
        },
        StarterTemplate {
            name: "demo-graph",
            content: DEMO_GRAPH,
        },
    ]
}

/// Convenience helper returning the names of every bundled template.
pub fn starter_template_names() -> Vec<&'static str> {
    starter_templates().iter().map(|t| t.name).collect()
}

const DAILY_NOTE: &str = r#"---
title: {{title}}
tags: [daily, journal]
date: {{date}}
---

# {{title}}

**Date:** {{date}}

## Focus

- 

## Log

- 

## Notes

- [[journal]]
- #daily quick capture

## Next Actions

- [ ] 
"#;

const MEETING_NOTES: &str = r#"---
title: {{title}}
tags: [meeting]
date: {{date}}
---

# {{title}}

**Date:** {{date}}

## Attendees

- 

## Agenda

1. 

## Decisions

- 

## Action Items

- [ ] TODO: 
- [ ] DOING: 

## Notes

- #meeting
- Follow-up tasks go into [[project-tracker]].
"#;

const READING_LIST: &str = r#"---
title: {{title}}
tags: [reading]
---

# {{title}}

## Reading List

| Title | Author | Status | Notes |
|-------|--------|--------|-------|
|       |        | To Read |       |

## Notes

- See [[book-notes]] for detailed reviews.
- #reading
"#;

const BOOK_NOTES: &str = r#"---
title: {{title}}
tags: [book]
author: {{author}}
status: {{status}}
---

# {{title}}

**Author:** {{author}}
**Status:** {{status}}

## Summary

## Key Takeaways

- 

## Quotes

> 

## Related

- #book
- Add to your [[reading-list]].
"#;

const PROJECT_TRACKER: &str = r#"---
title: {{title}}
tags: [project]
status: {{status}}
---

# {{title}}

## Goal

## Milestones

- [ ] TODO: 
- [ ] DOING: 
- [ ] DONE: 

## Status

## Risks

- 

## Related

- #project
- Track blockers in [[meeting-notes]].
"#;

const RESEARCH_NOTES: &str = r#"---
title: {{title}}
tags: [research]
---

# {{title}}

## Question

## Sources

- 

## Findings

- 

## Follow-ups

- 

## Notes

- #research
- See [[project-tracker]] for the related work.
"#;

const FLASHCARDS_STARTER: &str = r#"---
title: {{title}}
tags: [flashcards]
---

# {{title}}

- .question:: What is Stratum?
  .answer:: A privacy-first, offline-capable personal knowledge management system.
- .question:: What is a block?
  .answer:: An addressable paragraph with a UUID in the outliner.
- .question:: How do I link notes?
  .answer:: Use wiki-link syntax like [[daily-note]].
- #flashcards
"#;

const KANBAN_STARTER: &str = r#"---
title: {{title}}
tags: [kanban]
---

# {{title}}

- TODO: Backlog item one
  - TODO: Backlog sub-item
- DOING: In-progress item
- DONE: Completed item
- CANCELLED: Won't-do item
- #kanban
- Related board notes: [[project-tracker]].
"#;

const WHITEBOARD_STARTER: &str = r#"---
title: {{title}}
tags: [whiteboard]
---

# {{title}}

This note is a starting point for the Whiteboards panel. Open **Whiteboards**
from the sidebar to create an Excalidraw canvas, or link to
[[whiteboard-starter]] to reference it from other notes.

- #whiteboard
"#;

const HABIT_TRACKER: &str = r#"---
title: {{title}}
tags: [habit]
---

# {{title}}

## Weekly Grid

| Habit | Mon | Tue | Wed | Thu | Fri | Sat | Sun |
|-------|-----|-----|-----|-----|-----|-----|-----|
|       | ☐   | ☐   | ☐   | ☐   | ☐   | ☐   | ☐   |

## Log

- 

## Notes

- #habit
- Reflect weekly in [[daily-note]].
"#;

const QUICK_NOTES: &str = r#"---
title: {{title}}
tags: [notes]
---

# {{title}}

A scratch / inbox note for quick captures.

- 

## Ideas

- 

## Links

- #notes
- Related: [[meeting-notes]].
"#;

const DEMO_GRAPH: &str = r#"---
title: {{title}}
tags: [demo]
---

# {{title}}

A demo note demonstrating wiki-links, backlinks, and the knowledge graph.

- Link to [[daily-note]] to see a focused example.
- Link to [[meeting-notes]] for the meeting template.
- Link to [[project-tracker]] for a project template.
- #demo
"#;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn starter_templates_has_at_least_eight_entries() {
        assert!(
            starter_templates().len() >= 8,
            "expected at least 8 starter templates, got {}",
            starter_templates().len()
        );
    }

    #[test]
    fn every_template_name_is_a_valid_file_stem() {
        for t in starter_templates() {
            assert!(!t.name.is_empty(), "template name must not be empty");
            assert!(
                !t.name.contains('/') && !t.name.contains('\\'),
                "template name must be a file stem (no separators): {:?}",
                t.name
            );
            assert!(
                !t.name.contains('.'),
                "template name must not include an extension: {:?}",
                t.name
            );
            assert!(
                t.name
                    .bytes()
                    .all(|b| b.is_ascii_alphanumeric() || b == b'-'),
                "template name must be a slug (alphanumeric + dash): {:?}",
                t.name
            );
        }
    }

    #[test]
    fn every_template_content_has_frontmatter_heading_wikilink_and_tag() {
        for t in starter_templates() {
            assert!(
                t.content.starts_with("---\n"),
                "template {:?} content must start with YAML frontmatter '---\\n'",
                t.name
            );
            assert!(
                t.content.contains('#'),
                "template {:?} content must contain a '#' heading or tag token",
                t.name
            );
            assert!(
                t.content.contains("[["),
                "template {:?} content must contain a wiki-link '[[...]]'",
                t.name
            );
        }
    }

    #[test]
    fn starter_template_names_matches_templates() {
        let names = starter_template_names();
        assert_eq!(names.len(), starter_templates().len());
        assert_eq!(names[0], "daily-note");
        assert_eq!(names[1], "meeting-notes");
    }
}
