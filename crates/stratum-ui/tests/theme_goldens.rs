//! Theme derivation golden tests.
//!
//! The expected values were produced by running the shipped TypeScript theme
//! derivation through node. They pin the gpui port to the frozen React shell;
//! changing a formula here without changing the shipped formula is a parity
//! regression. The tests live in an integration test so the source module
//! stays under the repository's 500-line limit.

use stratum_ui::theme::{
    base_font_size, derive_palette, derive_shades, hex_to_hsl, hsl_css, parse_hsl_css,
    shade_offset, Hsl, ThemeConfig, ERROR_COLORS,
};

#[test]
fn hex_to_hsl_matches_the_typescript_goldens() {
    assert_eq!(
        hex_to_hsl("#f0883e"),
        Hsl {
            h: 25,
            s: 86,
            l: 59
        }
    );
    assert_eq!(
        hex_to_hsl("#3a2418"),
        Hsl {
            h: 21,
            s: 41,
            l: 16
        }
    );
    assert_eq!(
        hex_to_hsl("#9c27b0"),
        Hsl {
            h: 291,
            s: 64,
            l: 42
        }
    );
    assert_eq!(
        hex_to_hsl("#123"),
        Hsl {
            h: 210,
            s: 50,
            l: 13
        }
    );
}

#[test]
fn invalid_hex_falls_back_to_black() {
    // Documented fallback: "nope" matches neither shorthand nor six-digit
    // hex, so every channel stays zero like the JavaScript implementation.
    assert_eq!(hex_to_hsl("nope"), Hsl { h: 0, s: 0, l: 0 });
}

#[test]
fn shade_derivation_matches_the_typescript_goldens() {
    let primary = derive_shades("#f0883e");
    assert_eq!(
        primary.get(&500),
        Some(&Hsl {
            h: 25,
            s: 86,
            l: 59
        })
    );
    assert_eq!(
        primary.get(&50),
        Some(&Hsl {
            h: 25,
            s: 86,
            l: 100
        })
    );
    assert_eq!(
        primary.get(&900),
        Some(&Hsl {
            h: 25,
            s: 86,
            l: 25
        })
    );

    let secondary = derive_shades("#3a2418");
    assert_eq!(
        secondary.get(&500),
        Some(&Hsl {
            h: 21,
            s: 41,
            l: 16
        })
    );
    assert_eq!(secondary.get(&700), Some(&Hsl { h: 21, s: 41, l: 0 }));
    assert_eq!(
        secondary.get(&50),
        Some(&Hsl {
            h: 21,
            s: 41,
            l: 62
        })
    );

    assert_eq!(
        derive_shades("#9c27b0").get(&300),
        Some(&Hsl {
            h: 291,
            s: 64,
            l: 60
        })
    );
    assert_eq!(
        derive_shades("#123").get(&900),
        Some(&Hsl {
            h: 210,
            s: 50,
            l: 0
        })
    );
}

#[test]
fn shade_offset_is_lightness_minus_fifty() {
    assert_eq!(shade_offset(59), 9);
    assert_eq!(shade_offset(16), -34);
}

#[test]
fn dark_palette_matches_the_typescript_goldens() {
    let palette = derive_palette("#f0883e", "#3a2418", true);
    assert_eq!(palette.background_default, "hsl(21, 25%, 7%)");
    assert_eq!(palette.background_paper, "hsl(21, 20%, 12%)");
    assert_eq!(palette.divider, "hsl(21, 18%, 22%)");
    assert_eq!(palette.text_primary, "hsl(21, 10%, 92%)");
    assert_eq!(palette.text_secondary, "hsl(21, 8%, 68%)");
    assert_eq!(palette.text_disabled, "hsl(21, 6%, 38%)");
    assert_eq!(palette.primary_bg, "hsl(25, 86%, 25%)");
    assert_eq!(palette.primary_bg_hover, "hsl(25, 86%, 35%)");
    assert_eq!(palette.primary_text, "hsl(25, 86%, 77%)");
    assert_eq!(palette.primary_border, "hsl(25, 86%, 35%)");
}

#[test]
fn light_palette_matches_the_typescript_goldens() {
    let palette = derive_palette("#2196f3", "#607d8b", false);
    assert_eq!(palette.background_default, "hsl(200, 18%, 94%)");
    assert_eq!(palette.background_paper, "#ffffff");
    assert_eq!(palette.divider, "hsl(200, 18%, 82%)");
    assert_eq!(palette.text_primary, "hsl(200, 12%, 18%)");
    assert_eq!(palette.text_secondary, "hsl(200, 10%, 48%)");
    assert_eq!(palette.text_disabled, "hsl(200, 8%, 68%)");
    assert_eq!(palette.primary_bg, "hsl(207, 90%, 100%)");
    assert_eq!(palette.primary_text, "hsl(207, 90%, 46%)");
}

#[test]
fn neutral_palette_matches_the_typescript_goldens() {
    let palette = derive_palette("#000000", "#ffffff", true);
    assert_eq!(palette.background_default, "hsl(0, 0%, 7%)");
    assert_eq!(palette.text_primary, "hsl(0, 10%, 92%)");
}

#[test]
fn base_font_size_defaults() {
    assert_eq!(base_font_size(None), 16.0);
    assert_eq!(base_font_size(Some(0.0)), 16.0);
    assert_eq!(base_font_size(Some(18.0)), 18.0);
}

#[test]
fn hsl_css_round_trips() {
    let hsl = Hsl {
        h: 25,
        s: 86,
        l: 59,
    };
    assert_eq!(parse_hsl_css(&hsl_css(&hsl)), Some(hsl));
    assert_eq!(parse_hsl_css("nope"), None);
}

#[test]
fn default_theme_config_matches_the_shipped_defaults() {
    let cfg = ThemeConfig::default();
    assert_eq!(cfg.primary_hex, "#f0883e");
    assert_eq!(cfg.secondary_hex, "#3a2418");
    assert!(cfg.dark);
    assert_eq!(cfg.font_size, None);
    assert_eq!(cfg.palette(), derive_palette("#f0883e", "#3a2418", true));
}

#[test]
fn error_colors_are_the_mui_defaults() {
    assert_eq!(ERROR_COLORS.main, "#d32f2f");
    assert_eq!(ERROR_COLORS.light, "#ef5350");
    assert_eq!(ERROR_COLORS.dark, "#c62828");
}
