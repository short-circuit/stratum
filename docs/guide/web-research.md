# Web Research

Stratum can perform multi-depth web research and automatically integrate findings into your notes.

<!-- SCREENSHOT: [research-in-progress] Research panel showing search results being processed -->

## Prerequisites

You need a running [SearXNG](https://docs.searxng.org/) instance — a privacy-respecting metasearch engine.

### Setting Up SearXNG

```bash
# Using Docker
docker run --rm -d -p 8888:8080 searxng/searxng

# Or use docker-compose with custom configuration
```

### Configuration

1. Open **Settings → Research**
2. Set **SearXNG Endpoint** to your instance URL (default: `http://localhost:8888`)
3. Adjust **Max Results** (1–10) and **Research Depth** (1–5)
4. Click **Save**

## How Research Works

Stratum's research engine performs iterative depth-first search:

1. **Search** — query SearXNG for the topic
2. **Read** — extract content from the top results
3. **Deeper** — follow interesting links from results (configurable depth)
4. **Synthesize** — combine findings into a structured report

## Performing Research

Use the AI research feature from the slash menu or command palette:

1. Open any note
2. Trigger AI → **Research**
3. Enter your research question or topic
4. Stratum searches the web and synthesizes findings

<!-- SCREENSHOT: [research-results] Research results with findings and sources -->

## Research Results

Results include:

- **Findings** — synthesized summary of what was discovered
- **Sources** — list of web pages with titles, URLs, and snippets
- **Citations** — each finding references its source

## Tips

- **Be specific** in your research queries — narrower questions yield better results
- **Depth 1** is sufficient for fact-checking. **Depth 2–3** is for in-depth research
- Research findings can be inserted directly into your notes as block content
- Combine research with AI rewrite/summarize for polished reference notes
