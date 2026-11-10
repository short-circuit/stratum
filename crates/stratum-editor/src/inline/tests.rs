use super::*;

#[test]
fn round_trips_every_parsed_input_byte_identically() {
    let fixtures = [
        "",
        "plain text",
        "**bold**",
        "*italic*",
        "~~strike~~",
        "`code`",
        "==highlight==",
        "[[Target]]",
        "[[Target|display]]",
        "![[assets/x.png]]",
        "#tag",
        "a #tag/sub_tag",
        "$x + y$",
        "$$x = 1$$",
        "((65f8a1e2-3a4b-4c5d-6e7f-8a9b0c1d2e3f))",
        "[label](https://example.com/a)",
        "**unclosed",
        "*unclosed",
        "~~unclosed",
        "`unclosed",
        "[[unclosed",
        "\\*not italic\\*",
        "\\_not italic\\_",
        "\\`not code\\`",
        "\\[not a link\\]",
        "日本語のテキストと **太字** と #タグ",
        "emoji 😀 **太字** 😀",
        "nested **bold *italic* bold**",
        "a **b** c `d` e [[f]] g #h",
        "# Heading",
        "## Heading",
        "![alt](https://example.com/a.png)",
    ];
    for fixture in fixtures {
        let spans = parse_inline(fixture);
        assert_eq!(
            render_inline(&spans),
            fixture,
            "round-trip failed for {fixture:?}"
        );
    }
}

#[test]
fn parses_bold_and_italic() {
    let spans = parse_inline("**bold** and *italic*");
    let bold = spans
        .iter()
        .find(|span| span.kind == SpanKind::Bold)
        .expect("bold span");
    assert_eq!(bold.text, "bold");
    let italic = spans
        .iter()
        .find(|span| span.kind == SpanKind::Italic)
        .expect("italic span");
    assert_eq!(italic.text, "italic");
}

#[test]
fn parses_wiki_links_and_embeds() {
    let spans = parse_inline("[[Target|display]] ![[assets/x.png]]");
    assert_eq!(
        spans[0].kind,
        SpanKind::WikiLink {
            target: "Target".into(),
            embed: false
        }
    );
    assert_eq!(spans[0].text, "Target|display");
    assert_eq!(
        spans[2].kind,
        SpanKind::Asset {
            path: "assets/x.png".into()
        }
    );
}

#[test]
fn asset_embeds_classify_but_plain_asset_links_stay_wiki() {
    let embed = "![[assets/foo.png|My chart]]";
    let spans = parse_inline(embed);
    assert_eq!(spans.len(), 1);
    assert_eq!(
        spans[0].kind,
        SpanKind::Asset {
            path: "assets/foo.png".into()
        }
    );
    assert_eq!(render_inline(&spans), embed);

    let plain = "[[assets/bar.pdf]]";
    let spans = parse_inline(plain);
    assert_eq!(spans.len(), 1);
    assert_eq!(
        spans[0].kind,
        SpanKind::WikiLink {
            target: "assets/bar.pdf".into(),
            embed: false
        }
    );
    assert_eq!(render_inline(&spans), plain);

    let spaced = "![[ assets/foo.png ]]";
    let spans = parse_inline(spaced);
    assert_eq!(spans.len(), 1);
    assert_eq!(
        spans[0].kind,
        SpanKind::Asset {
            path: "assets/foo.png".into()
        }
    );
    assert_eq!(render_inline(&spans), spaced);
}

#[test]
fn parses_tags_but_not_heading_markers() {
    let tag = parse_inline("#tag");
    assert_eq!(tag[0].kind, SpanKind::Tag);
    assert_eq!(tag[0].text, "tag");

    let heading = parse_inline("# Heading");
    assert_eq!(heading[0].kind, SpanKind::Text);

    let mid_word = parse_inline("x#tag");
    assert_eq!(mid_word[0].kind, SpanKind::Text);
}

#[test]
fn parses_math_and_block_refs() {
    let inline = parse_inline("$x$");
    assert_eq!(inline[0].kind, SpanKind::Math { display: false });
    let display = parse_inline("$$x$$");
    assert_eq!(display[0].kind, SpanKind::Math { display: true });
    let block_ref = parse_inline("((abc))");
    assert_eq!(block_ref[0].kind, SpanKind::BlockRef { id: "abc".into() });
}

#[test]
fn parses_markdown_links() {
    let spans = parse_inline("[label](https://example.com)");
    assert_eq!(
        spans[0].kind,
        SpanKind::Link {
            url: "https://example.com".into()
        }
    );
    assert_eq!(spans[0].text, "label");
}

#[test]
fn escapes_are_kept_as_text() {
    let spans = parse_inline("\\*not italic\\*");
    assert!(spans.iter().all(|span| span.kind == SpanKind::Text));
}

#[test]
fn nested_markers_fall_back_to_inner_text() {
    let spans = parse_inline("**bold *italic* bold**");
    assert_eq!(spans.len(), 1);
    assert_eq!(spans[0].kind, SpanKind::Bold);
    assert_eq!(spans[0].text, "bold *italic* bold");
}

#[test]
fn unclosed_markers_degrade_to_text() {
    let spans = parse_inline("**bold");
    assert!(spans.iter().all(|span| span.kind == SpanKind::Text));
}

#[test]
fn byte_ranges_cover_the_source() {
    let text = "a **b** c";
    let spans = parse_inline(text);
    assert_eq!(spans[0].start_byte, 0);
    assert_eq!(spans.last().unwrap().end_byte, text.len());
    for span in &spans {
        assert!(span.start_byte < span.end_byte);
        assert!(span.end_byte <= text.len());
    }
}
