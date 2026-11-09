//! Unit tests for the caret and overlay token helpers.

use ::core::prelude::v1::test;

use super::overlay_ops::{marker_token_start, slash_token_start};
use super::{next_char_boundary, prev_char_boundary};

#[test]
fn slash_token_is_word_anchored() {
    assert_eq!(slash_token_start("hello /head", 11), Some(6));
    assert_eq!(slash_token_start("a/b", 3), None);
}

#[test]
fn marker_token_requires_a_non_empty_body() {
    assert_eq!(marker_token_start("task :todo", 10), Some(5));
    assert_eq!(marker_token_start("task:", 5), None);
}

#[test]
fn char_boundaries_stay_on_char_boundaries() {
    let text = "héllo";
    assert_eq!(prev_char_boundary(text, 2), 1);
    assert_eq!(next_char_boundary(text, 1), 3);
    assert_eq!(next_char_boundary(text, text.len()), text.len());
}

#[test]
fn caret_helpers_clamp_to_the_text_length() {
    assert_eq!(prev_char_boundary("", 0), 0);
    assert_eq!(next_char_boundary("", 0), 0);
}
