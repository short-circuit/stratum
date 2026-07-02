# Templates

Templates let you create reusable page structures with variable substitution.

<!-- SCREENSHOT: [templates-panel] Templates panel showing available templates -->

## Opening Templates

Click **📋 Templates** in the sidebar or navigate to `/templates`.

## Creating a Template

1. Open the **Templates** panel
2. Click **+ New**
3. Enter a template name (e.g., `meeting-notes`)
4. Write the template content with `{{variable}}` placeholders
5. Save

Example template:

```markdown
---
title: {{title}}
tags: [{{tags}}]
date: {{date}}
---

# {{title}}

## Attendees
{{attendees}}

## Agenda
1.

## Notes

## Action Items

## Next Meeting
```

## Template Variables

Templates support variable substitution with `{{variable_name}}` syntax:

| Built-in Variable | Description |
|-------------------|-------------|
| `{{title}}` | The page title (you provide when applying) |
| `{{date}}` | Today's date (auto-filled) |
| `{{time}}` | Current time (auto-filled) |
| Custom | Any variable name — you fill in the value |

## Applying a Template

1. Go to the page where you want to apply the template
2. Open **Templates**
3. Click **Apply** next to the template
4. Fill in the variable values
5. The template content is inserted into your page

<!-- SCREENSHOT: [template-apply] Variable substitution dialog when applying a template -->

## Template Storage

Templates are stored in your vault at `templates/`:

```
your-vault/
└── templates/
    ├── meeting-notes.md
    ├── project-plan.md
    └── book-note.md
```

You can create and edit template files directly. They're just Markdown files.

## Example Templates

### Book Note

```markdown
---
title: {{title}}
tags: [book, {{status}}]
author: {{author}}
---

# {{title}}

**Author:** {{author}}
**Status:** {{status}} / Reading / Done

## Summary

## Key Takeaways

## Quotes

## Related
[[{{related}}]]
```

### Project Plan

```markdown
---
title: {{title}}
tags: [project, planning]
status: {{status}}
---

# {{title}}: Project Plan

## Goal

## Milestones
1.
2.
3.

## Resources

## Risks

## Timeline
```

## Tips

- Create templates for recurring note types: meeting notes, book reviews, project plans, daily logs
- Use descriptive template names so they're easy to find
- Store templates in version control alongside your vault for team sharing
