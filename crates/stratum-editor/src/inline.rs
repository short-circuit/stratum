//! Inline span model, parser and byte-exact serializer.
//!
//! [`parse_inline`] recognizes the inline constructs the shipped editor stores in
//! block content: emphasis, code, highlight, wiki-links (including embeds),
//! Markdown links, tags, math and block references. [`render_inline`] is the
//! inverse for every input it accepts: `render_inline(parse_inline(s)) == s`.
//!
//! The parser is deliberately non-recursive ("non-nested"): the first matching
//! construct wins and its inner text is kept verbatim, so a marker nested inside
//! another marker degrades to plain inner text. Unclosed or unmatched markers
//! degrade to [`SpanKind::Text`] one scalar at a time, which also guarantees the
//! byte-exact round-trip for arbitrary input.
//!
//! Escapes (`\*`, `\_`, `` \` ``, `\[`, `\]`) are preserved verbatim as text;
//! the serializer does not unescape them.
//!
//! This module has no UI types and no locale strings.

/// The semantic kind of an inline span.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SpanKind {
    /// Unstyled text, including escape sequences kept verbatim.
    Text,
    /// `**bold**`.
    Bold,
    /// `*italic*`.
    Italic,
    /// `~~strike~~`.
    Strike,
    /// `` `code` ``.
    Code,
    /// `==highlight==`.
    Highlight,
    /// `[label](url)`.
    Link { url: String },
    /// `[[target]]` / `![[target]]`. `text` holds the raw inner text so a
    /// redundant `[[x|x]]` still round-trips byte-identically.
    WikiLink { target: String, embed: bool },
    /// `#tag`.
    Tag,
    /// `$inline$` or `$$display$$`.
    Math { display: bool },
    /// `((block-id))`.
    BlockRef { id: String },
    /// `![[path]]` asset embed. `text` holds the raw inner text so labels and
    /// surrounding whitespace round-trip byte-exactly.
    Asset { path: String },
}

/// One inline span with its byte range in the source string.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Span {
    pub kind: SpanKind,
    pub text: String,
    pub start_byte: usize,
    pub end_byte: usize,
}

impl Span {
    /// Whether `offset` falls inside this span (half-open byte range).
    pub fn contains(&self, offset: usize) -> bool {
        offset >= self.start_byte && offset < self.end_byte
    }
}

/// Parse `text` into non-overlapping inline spans covering the whole input.
pub fn parse_inline(text: &str) -> Vec<Span> {
    let mut spans = Vec::new();
    let mut cursor = 0usize;
    while cursor < text.len() {
        let ch = text[cursor..].chars().next().expect("cursor inside text");

        if ch == '\\' {
            if let Some(next) = text[cursor + 1..].chars().next() {
                if matches!(next, '*' | '_' | '`' | '[' | ']') {
                    let end = cursor + 1 + next.len_utf8();
                    spans.push(text_span(cursor, end, &text[cursor..end]));
                    cursor = end;
                    continue;
                }
            }
            let end = cursor + ch.len_utf8();
            spans.push(text_span(cursor, end, "\\"));
            cursor = end;
            continue;
        }

        if let Some((kind, inner, end)) = match_wikilink(text, cursor) {
            spans.push(Span {
                kind,
                text: inner,
                start_byte: cursor,
                end_byte: end,
            });
            cursor = end;
            continue;
        }

        if ch == '[' {
            if let Some((url, label, end)) = match_markdown_link(text, cursor) {
                spans.push(Span {
                    kind: SpanKind::Link { url },
                    text: label,
                    start_byte: cursor,
                    end_byte: end,
                });
                cursor = end;
                continue;
            }
        }

        if let Some((kind, inner, end)) = match_delimited(text, cursor, "**", SpanKind::Bold) {
            spans.push(Span {
                kind,
                text: inner,
                start_byte: cursor,
                end_byte: end,
            });
            cursor = end;
            continue;
        }

        if let Some((kind, inner, end)) = match_delimited(text, cursor, "~~", SpanKind::Strike) {
            spans.push(Span {
                kind,
                text: inner,
                start_byte: cursor,
                end_byte: end,
            });
            cursor = end;
            continue;
        }

        if let Some((kind, inner, end)) = match_delimited(text, cursor, "==", SpanKind::Highlight) {
            spans.push(Span {
                kind,
                text: inner,
                start_byte: cursor,
                end_byte: end,
            });
            cursor = end;
            continue;
        }

        if let Some((kind, inner, end)) = match_delimited(text, cursor, "`", SpanKind::Code) {
            spans.push(Span {
                kind,
                text: inner,
                start_byte: cursor,
                end_byte: end,
            });
            cursor = end;
            continue;
        }

        if let Some((kind, inner, end)) = match_delimited(text, cursor, "*", SpanKind::Italic) {
            spans.push(Span {
                kind,
                text: inner,
                start_byte: cursor,
                end_byte: end,
            });
            cursor = end;
            continue;
        }

        if let Some((kind, inner, end)) =
            match_delimited(text, cursor, "$$", SpanKind::Math { display: true })
        {
            spans.push(Span {
                kind,
                text: inner,
                start_byte: cursor,
                end_byte: end,
            });
            cursor = end;
            continue;
        }

        if let Some((kind, inner, end)) =
            match_delimited(text, cursor, "$", SpanKind::Math { display: false })
        {
            spans.push(Span {
                kind,
                text: inner,
                start_byte: cursor,
                end_byte: end,
            });
            cursor = end;
            continue;
        }

        if let Some((kind, id, end)) = match_block_ref(text, cursor) {
            spans.push(Span {
                kind,
                text: id,
                start_byte: cursor,
                end_byte: end,
            });
            cursor = end;
            continue;
        }

        if ch == '#' {
            if let Some((name, end)) = match_tag(text, cursor) {
                spans.push(Span {
                    kind: SpanKind::Tag,
                    text: name,
                    start_byte: cursor,
                    end_byte: end,
                });
                cursor = end;
                continue;
            }
        }

        let end = cursor + ch.len_utf8();
        spans.push(text_span(cursor, end, &text[cursor..end]));
        cursor = end;
    }
    spans
}

/// Serialize spans back to a byte-identical string for parsed input.
pub fn render_inline(spans: &[Span]) -> String {
    let mut out = String::new();
    for span in spans {
        match &span.kind {
            SpanKind::Text => out.push_str(&span.text),
            SpanKind::Bold => {
                out.push_str("**");
                out.push_str(&span.text);
                out.push_str("**");
            }
            SpanKind::Italic => {
                out.push('*');
                out.push_str(&span.text);
                out.push('*');
            }
            SpanKind::Strike => {
                out.push_str("~~");
                out.push_str(&span.text);
                out.push_str("~~");
            }
            SpanKind::Code => {
                out.push('`');
                out.push_str(&span.text);
                out.push('`');
            }
            SpanKind::Highlight => {
                out.push_str("==");
                out.push_str(&span.text);
                out.push_str("==");
            }
            SpanKind::Link { url } => {
                out.push('[');
                out.push_str(&span.text);
                out.push_str("](");
                out.push_str(url);
                out.push(')');
            }
            SpanKind::WikiLink { embed, .. } => {
                if *embed {
                    out.push_str("![[");
                } else {
                    out.push_str("[[");
                }
                // `text` is the raw inner text, so pipes and redundant display
                // segments survive byte-for-byte.
                out.push_str(&span.text);
                out.push_str("]]");
            }
            SpanKind::Tag => {
                out.push('#');
                out.push_str(&span.text);
            }
            SpanKind::Math { display } => {
                if *display {
                    out.push_str("$$");
                    out.push_str(&span.text);
                    out.push_str("$$");
                } else {
                    out.push('$');
                    out.push_str(&span.text);
                    out.push('$');
                }
            }
            SpanKind::BlockRef { .. } => {
                out.push_str("((");
                out.push_str(&span.text);
                out.push_str("))");
            }
            SpanKind::Asset { path } => {
                out.push_str("![[");
                // `text` is the raw inner text, so labels and surrounding
                // whitespace survive byte-for-byte.
                if span.text.is_empty() {
                    out.push_str(path);
                } else {
                    out.push_str(&span.text);
                }
                out.push_str("]]");
            }
        }
    }
    out
}

/// Find the span covering `offset`, if any.
pub fn span_at(spans: &[Span], offset: usize) -> Option<&Span> {
    spans.iter().find(|span| span.contains(offset))
}

fn text_span(start: usize, end: usize, text: &str) -> Span {
    Span {
        kind: SpanKind::Text,
        text: text.to_string(),
        start_byte: start,
        end_byte: end,
    }
}

/// Match `![[…]]` or `[[…]]` at `start`.
fn match_wikilink(text: &str, start: usize) -> Option<(SpanKind, String, usize)> {
    let rest = &text[start..];
    let (prefix, embed) = if rest.starts_with("![[") {
        ("![[", true)
    } else if rest.starts_with("[[") {
        ("[[", false)
    } else {
        return None;
    };
    let inner_start = start + prefix.len();
    let close = text[inner_start..].find("]]")?;
    let inner_end = inner_start + close;
    let inner = &text[inner_start..inner_end];
    let target = inner.split('|').next().unwrap_or(inner).trim().to_string();
    let end = inner_end + 2;
    // Only embeds (`![[…]]`) become asset spans. The plain `[[assets/…]]` form
    // stays a wiki link so it round-trips byte-identically and keeps its link
    // semantics.
    let kind = if embed && pkm_markdown::linker::is_asset_link_target(&target) {
        SpanKind::Asset { path: target }
    } else {
        SpanKind::WikiLink { target, embed }
    };
    Some((kind, inner.to_string(), end))
}

/// Match `[label](url)` at `start`.
fn match_markdown_link(text: &str, start: usize) -> Option<(String, String, usize)> {
    let rest = &text[start..];
    if !rest.starts_with('[') || rest.starts_with("[[") {
        return None;
    }
    let label_end_rel = rest[1..].find(']')?;
    let label_end = start + 1 + label_end_rel;
    if text.as_bytes().get(label_end + 1) != Some(&b'(') {
        return None;
    }
    let url_start = label_end + 2;
    let url_end_rel = text[url_start..].find(')')?;
    let url_end = url_start + url_end_rel;
    Some((
        text[url_start..url_end].to_string(),
        text[start + 1..label_end].to_string(),
        url_end + 1,
    ))
}

/// Match a delimiter-wrapped span with non-empty inner text.
fn match_delimited(
    text: &str,
    start: usize,
    delimiter: &str,
    kind: SpanKind,
) -> Option<(SpanKind, String, usize)> {
    let rest = &text[start..];
    if !rest.starts_with(delimiter) {
        return None;
    }
    let inner_start = start + delimiter.len();
    let close = text[inner_start..].find(delimiter)?;
    let inner_end = inner_start + close;
    let inner = &text[inner_start..inner_end];
    // An empty inner (`**`, `$$`, `""`) is not a span: leave both delimiters to
    // degrade to plain text instead of producing a zero-width styled span.
    if inner.is_empty() {
        return None;
    }
    let end = inner_end + delimiter.len();
    Some((kind, inner.to_string(), end))
}

/// Match `((block-id))` at `start`.
fn match_block_ref(text: &str, start: usize) -> Option<(SpanKind, String, usize)> {
    let rest = &text[start..];
    if !rest.starts_with("((") {
        return None;
    }
    let inner_start = start + 2;
    let close = text[inner_start..].find("))")?;
    let inner_end = inner_start + close;
    let id = &text[inner_start..inner_end];
    if id.is_empty() {
        return None;
    }
    let end = inner_end + 2;
    Some((
        SpanKind::BlockRef { id: id.to_string() },
        id.to_string(),
        end,
    ))
}

/// Match `#tag` at `start`, requiring a word boundary before the `#` and an
/// ASCII-letter first character after it (so an ATX heading marker is not a tag).
fn match_tag(text: &str, start: usize) -> Option<(String, usize)> {
    let rest = &text[start..];
    if !rest.starts_with('#') {
        return None;
    }
    let after = rest[1..].chars().next()?;
    if !after.is_ascii_alphabetic() {
        return None;
    }
    if start > 0 {
        let prev = text[..start].chars().next_back()?;
        if !prev.is_whitespace() && prev != '(' {
            return None;
        }
    }
    let name: String = rest[1..]
        .chars()
        .take_while(|c| c.is_ascii_alphanumeric() || matches!(c, '_' | '-' | '/'))
        .collect();
    let end = start + 1 + name.len();
    Some((name, end))
}

#[cfg(test)]
mod tests;
