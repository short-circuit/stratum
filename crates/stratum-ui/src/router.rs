//! Route, navigation and shortcut tables for the gpui shell.
//!
//! These tables are the single source of truth for shell navigation. They
//! mirror the shipped React router (`src/App.tsx`) and the sidebar navigation
//! list (`src/components/Sidebar/NavItemList.tsx`) so the gpui shell reaches
//! exactly the same thirteen panels in the same order. Keeping the tables as
//! plain data makes the layout contract testable without opening a window.

use gpui_kit::assets::IconName;
use gpui_kit::component::{Icon, Sizable};
use gpui_kit::prelude::*;
use gpui_kit::*;

/// A top-level shell panel.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Panel {
    /// Daily journal.
    Journal,
    /// Individual page editor.
    Page,
    /// Full-text search.
    Search,
    /// Datalog query builder.
    Query,
    /// AI Q&A over the vault.
    AskNotes,
    /// Page templates.
    Templates,
    /// SRS flashcards.
    Flashcards,
    /// Kanban board.
    Kanban,
    /// Block reminders.
    Reminders,
    /// Excalidraw whiteboards.
    Whiteboards,
    /// Force-directed graph.
    Graph,
    /// WASM plugins.
    Plugins,
    /// Application settings.
    Settings,
}

impl Panel {
    /// Every panel, in route-table order.
    pub const ALL: [Panel; 13] = [
        Panel::Journal,
        Panel::Page,
        Panel::Search,
        Panel::Query,
        Panel::AskNotes,
        Panel::Templates,
        Panel::Flashcards,
        Panel::Kanban,
        Panel::Reminders,
        Panel::Whiteboards,
        Panel::Graph,
        Panel::Plugins,
        Panel::Settings,
    ];

    /// Stable panel id, matching the shipped navigation ids where applicable.
    pub fn id(self) -> &'static str {
        match self {
            Panel::Journal => "journal",
            Panel::Page => "page",
            Panel::Search => "search",
            Panel::Query => "query",
            Panel::AskNotes => "asknotes",
            Panel::Templates => "templates",
            Panel::Flashcards => "flashcards",
            Panel::Kanban => "kanban",
            Panel::Reminders => "reminders",
            Panel::Whiteboards => "whiteboards",
            Panel::Graph => "graph",
            Panel::Plugins => "plugins",
            Panel::Settings => "settings",
        }
    }

    /// Localized title key used by the stub panels.
    pub fn title_key(self) -> &'static str {
        match self {
            Panel::Journal => "mobile_nav.journal",
            Panel::Page => "page.note",
            Panel::Search => "mobile_nav.search",
            Panel::Query => "mobile_nav.query",
            Panel::AskNotes => "mobile_nav.ask_notes",
            Panel::Templates => "mobile_nav.templates",
            Panel::Flashcards => "mobile_nav.flashcards",
            Panel::Kanban => "mobile_nav.kanban",
            Panel::Reminders => "mobile_nav.reminders",
            Panel::Whiteboards => "mobile_nav.whiteboards",
            Panel::Graph => "mobile_nav.graph",
            Panel::Plugins => "mobile_nav.plugins",
            Panel::Settings => "mobile_nav.settings",
        }
    }
}

/// A route pattern and the panel it opens.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Route {
    /// Panel opened by this route.
    pub panel: Panel,
    /// Shipped route pattern (`/page/:path` for the parameterized editor).
    pub path: &'static str,
}

/// The thirteen shipped routes. `"/"` is a redirect to the journal and is
/// handled by [`parse_path`], so it is not a separate route entry.
pub const ROUTES: [Route; 13] = [
    Route {
        panel: Panel::Journal,
        path: "/journal",
    },
    Route {
        panel: Panel::Page,
        path: "/page/:path",
    },
    Route {
        panel: Panel::Search,
        path: "/search",
    },
    Route {
        panel: Panel::Query,
        path: "/query",
    },
    Route {
        panel: Panel::AskNotes,
        path: "/ask-notes",
    },
    Route {
        panel: Panel::Templates,
        path: "/templates",
    },
    Route {
        panel: Panel::Flashcards,
        path: "/flashcards",
    },
    Route {
        panel: Panel::Kanban,
        path: "/kanban",
    },
    Route {
        panel: Panel::Reminders,
        path: "/reminders",
    },
    Route {
        panel: Panel::Whiteboards,
        path: "/whiteboards",
    },
    Route {
        panel: Panel::Graph,
        path: "/graph",
    },
    Route {
        panel: Panel::Plugins,
        path: "/plugins",
    },
    Route {
        panel: Panel::Settings,
        path: "/settings",
    },
];

/// A sidebar navigation row.
#[derive(Clone, Copy, Debug)]
pub struct NavItem {
    /// Shipped navigation id.
    pub id: &'static str,
    /// Panel opened by the row.
    pub panel: Panel,
    /// Shipped path.
    pub path: &'static str,
    /// Localized label key.
    pub label_key: &'static str,
    /// Icon renderer (20 px Lucide icon).
    pub icon: fn() -> gpui::AnyElement,
}

/// The twelve shipped navigation rows, in exactly the shipped order.
pub const NAV_ITEMS: [NavItem; 12] = [
    NavItem {
        id: "journal",
        panel: Panel::Journal,
        path: "/journal",
        label_key: "mobile_nav.journal",
        icon: journal_icon,
    },
    NavItem {
        id: "graph",
        panel: Panel::Graph,
        path: "/graph",
        label_key: "mobile_nav.graph",
        icon: graph_icon,
    },
    NavItem {
        id: "kanban",
        panel: Panel::Kanban,
        path: "/kanban",
        label_key: "mobile_nav.kanban",
        icon: kanban_icon,
    },
    NavItem {
        id: "reminders",
        panel: Panel::Reminders,
        path: "/reminders",
        label_key: "mobile_nav.reminders",
        icon: reminders_icon,
    },
    NavItem {
        id: "search",
        panel: Panel::Search,
        path: "/search",
        label_key: "mobile_nav.search",
        icon: search_icon,
    },
    NavItem {
        id: "query",
        panel: Panel::Query,
        path: "/query",
        label_key: "mobile_nav.query",
        icon: query_icon,
    },
    NavItem {
        id: "asknotes",
        panel: Panel::AskNotes,
        path: "/ask-notes",
        label_key: "mobile_nav.ask_notes",
        icon: ask_notes_icon,
    },
    NavItem {
        id: "templates",
        panel: Panel::Templates,
        path: "/templates",
        label_key: "mobile_nav.templates",
        icon: templates_icon,
    },
    NavItem {
        id: "flashcards",
        panel: Panel::Flashcards,
        path: "/flashcards",
        label_key: "mobile_nav.flashcards",
        icon: flashcards_icon,
    },
    NavItem {
        id: "whiteboards",
        panel: Panel::Whiteboards,
        path: "/whiteboards",
        label_key: "mobile_nav.whiteboards",
        icon: whiteboards_icon,
    },
    NavItem {
        id: "plugins",
        panel: Panel::Plugins,
        path: "/plugins",
        label_key: "mobile_nav.plugins",
        icon: plugins_icon,
    },
    NavItem {
        id: "settings",
        panel: Panel::Settings,
        path: "/settings",
        label_key: "mobile_nav.settings",
        icon: settings_icon,
    },
];

/// A keyboard shortcut action.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ShortcutAction {
    /// Toggle the page read mode (shipped Ctrl/Cmd+Shift+R).
    ToggleReadMode,
    /// Dismiss the topmost toast or error bar (Escape).
    DismissOverlay,
    /// Collapse or expand the sidebar (Ctrl/Cmd+B).
    ToggleSidebar,
}

/// A keyboard modifier.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Modifier {
    /// Control (mapped to `secondary` so macOS uses Command).
    Ctrl,
    /// Shift.
    Shift,
    /// Alt/Option.
    Alt,
    /// Meta/Command.
    Meta,
}

/// A keyboard shortcut row.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Shortcut {
    /// Action dispatched by the shortcut.
    pub action: ShortcutAction,
    /// Required modifiers.
    pub modifiers: &'static [Modifier],
    /// Key name (GPUI keystroke syntax).
    pub key: &'static str,
    /// Optional localized label key.
    pub label_key: Option<&'static str>,
}

/// The shipped shortcut table plus the sidebar toggle.
pub const SHORTCUTS: [Shortcut; 3] = [
    Shortcut {
        action: ShortcutAction::ToggleReadMode,
        modifiers: &[Modifier::Ctrl, Modifier::Shift],
        key: "r",
        label_key: Some("page.read_mode"),
    },
    Shortcut {
        action: ShortcutAction::DismissOverlay,
        modifiers: &[],
        key: "escape",
        label_key: None,
    },
    Shortcut {
        action: ShortcutAction::ToggleSidebar,
        modifiers: &[Modifier::Ctrl],
        key: "b",
        label_key: None,
    },
];

/// Build the GPUI keystroke string for a shortcut.
///
/// `Modifier::Ctrl` maps to GPUI's `secondary` modifier, which resolves to
/// Command on macOS and Control elsewhere. This is the same modifier the
/// shipped React shell uses for its Ctrl/Cmd shortcuts.
pub fn keystroke_string(shortcut: &Shortcut) -> String {
    let mut parts: Vec<&str> = shortcut
        .modifiers
        .iter()
        .map(|modifier| match modifier {
            Modifier::Ctrl => "secondary",
            Modifier::Shift => "shift",
            Modifier::Alt => "alt",
            Modifier::Meta => "meta",
        })
        .collect();
    parts.push(shortcut.key);
    parts.join("-")
}

/// Parse a URL path into a panel. `"/"` redirects to the journal.
pub fn parse_path(path: &str) -> Option<Panel> {
    if path == "/" {
        return Some(Panel::Journal);
    }
    if path == "/page" {
        return Some(Panel::Page);
    }
    // The shipped React app registers Ask Notes at `/ask`; the native router
    // also accepts the `/ask-notes` spelling used by the navigation table.
    if path == "/ask" {
        return Some(Panel::AskNotes);
    }
    if let Some(rest) = path.strip_prefix("/page/") {
        let _ = percent_decode(rest);
        return Some(Panel::Page);
    }
    ROUTES
        .iter()
        .find(|route| route.path == path)
        .map(|route| route.panel)
}

/// Build the path for a panel. `Page` uses `page_path` when provided.
pub fn path_for(panel: Panel, page_path: Option<&str>) -> String {
    if panel == Panel::Page {
        return match page_path {
            Some(path) => format!("/page/{}", percent_encode(path)),
            None => "/page".to_string(),
        };
    }
    ROUTES
        .iter()
        .find(|route| route.panel == panel)
        .map(|route| route.path.to_string())
        .unwrap_or_else(|| "/journal".to_string())
}

/// Percent-decode a path component. Invalid escapes are kept verbatim.
pub fn percent_decode(s: &str) -> String {
    let bytes = s.as_bytes();
    let mut out = Vec::with_capacity(bytes.len());
    let mut index = 0;
    while index < bytes.len() {
        if bytes[index] == b'%' && index + 2 < bytes.len() {
            let high = hex_value(bytes[index + 1]);
            let low = hex_value(bytes[index + 2]);
            if let (Some(high), Some(low)) = (high, low) {
                out.push(high * 16 + low);
                index += 3;
                continue;
            }
        }
        out.push(bytes[index]);
        index += 1;
    }
    // The input is a path component; invalid UTF-8 is replaced rather than
    // rejected so navigation can never panic.
    String::from_utf8_lossy(&out).into_owned()
}

/// Percent-encode a path component using RFC 3986 unreserved characters.
pub fn percent_encode(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for byte in s.bytes() {
        let unreserved = byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_' | b'.' | b'~');
        if unreserved {
            out.push(byte as char);
        } else {
            out.push_str(&format!("%{byte:02X}"));
        }
    }
    out
}

/// Decode one hexadecimal digit.
fn hex_value(byte: u8) -> Option<u8> {
    match byte {
        b'0'..=b'9' => Some(byte - b'0'),
        b'a'..=b'f' => Some(byte - b'a' + 10),
        b'A'..=b'F' => Some(byte - b'A' + 10),
        _ => None,
    }
}

/// Shared icon builder: 20 px, inheriting the caller's text colour.
fn nav_icon(name: IconName) -> gpui::AnyElement {
    Icon::new(name).with_size(px(20.0)).into_any_element()
}

fn journal_icon() -> gpui::AnyElement {
    nav_icon(IconName::Calendar)
}
fn graph_icon() -> gpui::AnyElement {
    nav_icon(IconName::Network)
}
fn kanban_icon() -> gpui::AnyElement {
    nav_icon(IconName::ListChecks)
}
fn reminders_icon() -> gpui::AnyElement {
    nav_icon(IconName::AlarmClock)
}
fn search_icon() -> gpui::AnyElement {
    nav_icon(IconName::Search)
}
fn query_icon() -> gpui::AnyElement {
    nav_icon(IconName::Code)
}
fn ask_notes_icon() -> gpui::AnyElement {
    nav_icon(IconName::Sparkles)
}
fn templates_icon() -> gpui::AnyElement {
    nav_icon(IconName::FileText)
}
fn flashcards_icon() -> gpui::AnyElement {
    nav_icon(IconName::CircleQuestionMark)
}
fn whiteboards_icon() -> gpui::AnyElement {
    nav_icon(IconName::PenTool)
}
fn plugins_icon() -> gpui::AnyElement {
    nav_icon(IconName::Puzzle)
}
fn settings_icon() -> gpui::AnyElement {
    nav_icon(IconName::Settings)
}
