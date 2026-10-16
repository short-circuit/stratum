//! Integration tests for [`pkm_markdown::linker::rewrite_link_targets`].
//!
//! Sits in `tests/` (not inline in `linker.rs`) to keep that source file under the
//! 500-line cap while still exercising the public rewrite API end-to-end.

use pkm_markdown::linker::rewrite_link_targets;

#[test]
fn rewrites_exact_name_link() {
    let (out, changed) = rewrite_link_targets("[[b]]", &[("b", "c")]);
    assert_eq!(out, "[[c]]");
    assert!(changed);
}

#[test]
fn rewrites_uppercase_name_link_capitalizes() {
    let (out, changed) = rewrite_link_targets("[[B]]", &[("b", "c")]);
    assert_eq!(out, "[[C]]");
    assert!(changed);
}

#[test]
fn rewrites_piped_link_preserves_display() {
    let (out, changed) = rewrite_link_targets("[[b|Display]]", &[("b", "c")]);
    assert_eq!(out, "[[c|Display]]");
    assert!(changed);
}

#[test]
fn preserves_display_with_escaped_pipe() {
    let (out, changed) = rewrite_link_targets("[[b|A\\|B]]", &[("b", "c")]);
    assert_eq!(out, "[[c|A\\|B]]");
    assert!(changed);
}

#[test]
fn rewrites_target_with_escaped_pipe() {
    let (out, changed) = rewrite_link_targets("[[b\\|x|D]]", &[("b|x", "c|x")]);
    assert_eq!(out, "[[c\\|x|D]]");
    assert!(changed);
}

#[test]
fn does_not_prefix_corrupt() {
    let text = "[[pages/b]] [[pages/bb]]";
    let (out, changed) = rewrite_link_targets(text, &[("pages/b", "pages/archive/b")]);
    assert_eq!(out, "[[pages/archive/b]] [[pages/bb]]");
    assert!(changed);
}

#[test]
fn preserves_nested_brackets_when_no_match() {
    let text = "[[Outer [[Inner]] rest]]";
    let (out, changed) = rewrite_link_targets(text, &[("b", "c")]);
    assert_eq!(out, text);
    assert!(!changed);
}

#[test]
fn no_match_returns_original_false() {
    let text = "See [[Other]] here.";
    let (out, changed) = rewrite_link_targets(text, &[("b", "c")]);
    assert_eq!(out, text);
    assert!(!changed);
}

#[test]
fn empty_text_is_noop() {
    let (out, changed) = rewrite_link_targets("", &[("b", "c")]);
    assert_eq!(out, "");
    assert!(!changed);
}
