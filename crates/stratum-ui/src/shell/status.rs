//! Status bar at the bottom of the shell.
//!
//! The React desktop shell does not draw a status bar, but the native shell
//! needs a stable surface for the build identity and the active panel. The bar
//! is intentionally quiet: the brand label on the left and the localized panel
//! title plus version on the right.

use gpui_kit::component::ActiveTheme;
use gpui_kit::*;
use stratum_i18n::t_global;

use crate::shell::Shell;

/// Height of the status bar.
pub const STATUS_HEIGHT: f32 = 24.0;

/// Render the status bar.
pub fn status(shell: &Shell, cx: &mut Context<Shell>) -> impl IntoElement {
    let theme = cx.theme();
    let background = theme.colors.secondary;
    let border = theme.colors.border;
    let muted = theme.colors.muted_foreground;

    let panel = t_global(shell.active_panel().title_key());

    div()
        .flex()
        .items_center()
        .justify_between()
        .h(px(STATUS_HEIGHT))
        .px(px(12.0))
        .bg(background)
        .border_t_1()
        .border_color(border)
        .text_size(px(11.0))
        .text_color(muted)
        .child(
            div()
                .flex()
                .items_center()
                .gap(px(8.0))
                .child(t_global("page.stratum_pkm"))
                .child(format!("v{}", env!("CARGO_PKG_VERSION"))),
        )
        .child(panel)
}

#[cfg(test)]
mod tests {
    use super::*;
    // `gpui_kit::*` re-exports gpui's `test` attribute macro, which shadows
    // the built-in attribute. Import the built-in explicitly so these plain
    // unit tests stay cheap (no GPUI app context is created).
    use ::core::prelude::v1::test;

    #[test]
    fn status_height_is_compact() {
        assert_eq!(STATUS_HEIGHT, 24.0);
    }
}
