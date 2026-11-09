use super::*;

fn source(latex: &str) -> MathSource {
    MathSource {
        latex: latex.to_string(),
        display: false,
    }
}

#[test]
fn extracts_inline_display_and_fenced_math() {
    let inline = math_source("$x^2$").expect("inline");
    assert_eq!(inline.latex, "x^2");
    assert!(!inline.display);

    let display = math_source("$$\\frac{1}{2}$$").expect("display");
    assert_eq!(display.latex, "\\frac{1}{2}");
    assert!(display.display);

    let fenced = math_source("```math\n\\sum_{i=0}^n i\n```").expect("fenced");
    assert_eq!(fenced.latex, "\\sum_{i=0}^n i");
    assert!(fenced.display);
}

#[test]
fn finds_inline_math_inside_plain_content() {
    let found = math_source("the value is $a+b$ here").expect("inline");
    assert_eq!(found.latex, "a+b");
    assert!(!found.display);
    assert!(math_source("plain text").is_none());
}

#[test]
fn renders_a_png_with_the_expected_signature() {
    let mut cache = MathCache::new();
    let image = cache
        .render(&source("x^2"), &MathOptions::default())
        .expect("render");
    assert_eq!(&image.png[..8], b"\x89PNG\r\n\x1a\n");
    assert!(image.width > 0);
    assert!(image.height > 0);
}

#[test]
fn renders_cjk_text() {
    let mut cache = MathCache::new();
    let image = cache
        .render(&source("\\text{你好}"), &MathOptions::default())
        .expect("cjk render");
    assert_eq!(&image.png[..8], b"\x89PNG\r\n\x1a\n");
}

#[test]
fn invalid_latex_is_a_render_error() {
    let mut cache = MathCache::new();
    let error = cache
        .render(&source("\\frac{1}{"), &MathOptions::default())
        .expect_err("invalid latex must fail");
    assert!(matches!(error, PkmError::Render(_)));
}

#[test]
fn cache_hits_misses_and_evicts() {
    let mut cache = MathCache::with_capacity(2);
    let options = MathOptions::default();
    cache.render(&source("a"), &options).expect("first");
    cache.render(&source("a"), &options).expect("cached");
    assert_eq!(cache.hits(), 1);
    assert_eq!(cache.misses(), 1);
    assert_eq!(cache.len(), 1);

    cache.render(&source("b"), &options).expect("second");
    cache.render(&source("c"), &options).expect("third");
    assert!(cache.len() <= 2);
    assert_eq!(cache.misses(), 3);
}

#[test]
fn cache_key_includes_render_options() {
    let a = cache_key(&source("x"), &MathOptions::default());
    let b = cache_key(
        &source("x"),
        &MathOptions {
            font_size: 24.0,
            ..MathOptions::default()
        },
    );
    assert_ne!(a, b);
}
